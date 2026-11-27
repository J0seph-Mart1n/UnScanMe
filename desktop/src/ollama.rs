//! Image -> HTML conversion using a vision model served by a local Ollama instance.

use crate::settings::Settings;
use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::Duration;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    options: ChatOptions,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    images: Vec<String>,
}

#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
    num_ctx: u32,
}

#[derive(Deserialize)]
struct ChatStreamChunk {
    #[serde(default)]
    message: Option<StreamMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct StreamMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    thinking: String,
}

/// Phase reported while streaming a response from Ollama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatPhase {
    Waiting,
    Thinking,
    Generating,
}

/// Live progress update emitted during Ollama chat generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatProgress {
    pub phase: ChatPhase,
    pub chars: usize,
    pub thinking_chars: usize,
    pub elapsed_secs: u64,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
}

#[derive(Deserialize)]
struct PullStreamLine {
    #[serde(default)]
    status: String,
    completed: Option<u64>,
    total: Option<u64>,
    error: Option<String>,
}

/// Live progress update emitted while pulling a model from Ollama.
#[derive(Debug, Clone, PartialEq)]
pub struct PullUpdate {
    pub text: String,
    pub percent: Option<u32>,
}

fn client(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().timeout(timeout).build()?)
}

/// Decode an image, downscale it so its longest side is at most `max_dim`,
/// and re-encode it as PNG (Ollama reliably accepts PNG/JPEG only).
pub fn prepare_image(bytes: &[u8], max_dim: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    let img = if img.width().max(img.height()) > max_dim {
        img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Cursor::new(Vec::new());
    img.to_rgb8().write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

const DOCUMENT_GENERATION_PROMPT: &str = r#"You are an expert document designer and HTML5 generator.
Create a SINGLE, complete, self-contained HTML5 document that fulfills the user's request cleanly and professionally.

Follow these rules strictly:
1. **Complete & Realistic Content**: Generate all requested sections, headings, dates, reference numbers, itemized tables, quantities, unit prices, subtotals, taxes (if appropriate), and accurately calculated totals.
2. **Layout, Page Setup & Visual Editor Compatibility**: Use clean, discrete semantic HTML5 elements (`<header>`, `<section>`, `<h1>`–`<h3>`, `<p>`, `<div>`, `<table>`) and put all CSS inside a single `<style>` block in `<head>`. Include `*, *::before, *::after { box-sizing: border-box; }` and `@page { size: A4 portrait; margin: 15mm; }` so the layout fits cleanly and adapts when toolbar Page Setup overrides paper size, orientation, or margins.
3. **Interactive Table Formatting**: For all tabular data, always use real semantic `<table>`, `<thead>`, `<tbody>`, `<tr>`, `<th>`, and `<td>` elements (never div/flexbox fake tables) styled with `border-collapse: collapse`, clear cell borders, readable padding, and right-aligned numbers/currency.
4. **No External Resources**: Do NOT use external `<img>` URLs, `<script>` tags, or external fonts. Use inline `<svg>` or CSS shapes only if a simple logo/divider is helpful.
5. **Output Format**: Return ONLY the raw HTML code starting with `<!DOCTYPE html>` and ending with `</html>`. Do NOT wrap the output in markdown code fences or include explanatory text."#;

const REFINE_HTML_PROMPT: &str = r#"You are an expert HTML5 document editor.
You will receive an existing self-contained HTML5 document and a user instruction requesting a modification.

Follow these rules strictly:
1. **Targeted Changes**: Apply ONLY the requested change(s) cleanly while preserving all existing text, data, layout, and styling that were not asked to change.
2. **Structure & Table Consistency**: Keep calculations (subtotals, taxes, totals), semantic `<table>` (`<thead>`, `<tbody>`, `<th>`, `<td>`) structure, Visual Editor-friendly elements, and `@page` print CSS consistent after your edit.
3. **No External Resources**: Do NOT introduce external URLs, external fonts, or `<script>` tags. Preserve any existing inline SVG or `data:` image URLs unless told to change them.
4. **Output Format**: Return ONLY the complete, updated raw HTML code starting with `<!DOCTYPE html>` and ending with `</html>`. Do NOT wrap in markdown code fences or include any explanatory text."#;

fn contains_closing_html(s: &str) -> bool {
    s.to_ascii_lowercase().contains("</html>")
}

/// Detect degenerate repetition loops at the tail of streaming model output.
/// Returns a valid UTF-8 byte index to truncate to if a loop is found.
pub fn detect_repetition_loop(s: &str) -> Option<usize> {
    const TAIL_WINDOW: usize = 1500;
    let mut start = s.len().saturating_sub(TAIL_WINDOW);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    let tail = &s[start..];
    if tail.len() < 240 {
        return None;
    }

    if let Some(last_ch) = tail.chars().next_back() {
        let mut run_chars = 0usize;
        let mut run_bytes = 0usize;
        for c in tail.chars().rev() {
            if c == last_ch {
                run_chars += 1;
                run_bytes += c.len_utf8();
            } else {
                break;
            }
        }
        if run_chars >= 250 {
            return Some(s.len() - run_bytes + last_ch.len_utf8());
        }
    }

    let tail_bytes = tail.as_bytes();
    let tail_len = tail_bytes.len();
    for p in 4..=120 {
        if p > tail_len || !tail.is_char_boundary(tail_len - p) {
            continue;
        }
        let pat = &tail_bytes[tail_len - p..];
        let mut repeats = 1usize;
        while (repeats + 1) * p <= tail_len {
            let next_start = tail_len - (repeats + 1) * p;
            if &tail_bytes[next_start..next_start + p] == pat {
                repeats += 1;
            } else {
                break;
            }
        }
        if repeats >= 12 && repeats * p >= 240 {
            return Some(s.len() - (repeats - 2) * p);
        }
    }

    None
}

async fn send_chat_streaming<F>(
    settings: &Settings,
    prompt: &str,
    images: Vec<String>,
    mut on_progress: F,
) -> Result<String>
where
    F: FnMut(ChatProgress),
{
    let url = format!("{}/api/chat", settings.ollama_url.trim_end_matches('/'));
    let request = ChatRequest {
        model: &settings.model,
        messages: vec![ChatMessage { role: "user", content: prompt, images }],
        stream: true,
        options: ChatOptions {
            temperature: settings.temperature,
            num_ctx: settings.num_ctx,
        },
    };

    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let started = tokio::time::Instant::now();
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut response = {
        let send_fut = std::pin::pin!(http.post(&url).json(&request).send());
        let mut send_fut = send_fut;
        loop {
            tokio::select! {
                res = &mut send_fut => {
                    break res.with_context(|| {
                        format!("could not reach Ollama at {url}. Is `ollama serve` running?")
                    })?;
                }
                _ = ticker.tick() => {
                    let elapsed = started.elapsed();
                    on_progress(ChatProgress {
                        phase: ChatPhase::Waiting,
                        chars: 0,
                        thinking_chars: 0,
                        elapsed_secs: elapsed.as_secs(),
                    });
                    if elapsed >= Duration::from_secs(180) {
                        bail!(
                            "Timed out after 180s waiting for `{}` to start responding. Try lowering Max image px in Settings, using Fast Convert (OCR), or checking `ollama ps`.",
                            settings.model
                        );
                    }
                }
            }
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Ollama returned {status}: {body}");
    }

    let mut last_activity = tokio::time::Instant::now();
    let mut last_emit = tokio::time::Instant::now();
    let mut accumulated = String::new();
    let mut thinking_chars = 0usize;
    let mut buffer = String::new();
    let mut loop_trimmed = false;

    'stream: loop {
        tokio::select! {
            chunk_res = response.chunk() => {
                match chunk_res {
                    Ok(Some(chunk)) => {
                        last_activity = tokio::time::Instant::now();
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer.drain(..=pos);
                            if line.is_empty() {
                                continue;
                            }
                            let Ok(event) = serde_json::from_str::<ChatStreamChunk>(&line) else {
                                continue;
                            };
                            if let Some(err) = event.error {
                                bail!("Ollama error: {err}");
                            }
                            if let Some(msg) = event.message {
                                thinking_chars += msg.thinking.len();
                                accumulated.push_str(&msg.content);
                            }
                            if last_emit.elapsed() >= Duration::from_millis(150) {
                                let phase = if !accumulated.is_empty() {
                                    ChatPhase::Generating
                                } else if thinking_chars > 0 {
                                    ChatPhase::Thinking
                                } else {
                                    ChatPhase::Waiting
                                };
                                on_progress(ChatProgress {
                                    phase,
                                    chars: accumulated.len(),
                                    thinking_chars,
                                    elapsed_secs: started.elapsed().as_secs(),
                                });
                                last_emit = tokio::time::Instant::now();
                            }
                            if contains_closing_html(&accumulated) {
                                break 'stream;
                            }
                            if let Some(cut_at) = detect_repetition_loop(&accumulated) {
                                accumulated.truncate(cut_at);
                                loop_trimmed = true;
                                break 'stream;
                            }
                            if event.done {
                                break 'stream;
                            }
                        }
                    }
                    Ok(None) => {
                        let trailing = buffer.trim();
                        if !trailing.is_empty() {
                            if let Ok(event) = serde_json::from_str::<ChatStreamChunk>(trailing) {
                                if let Some(err) = event.error {
                                    bail!("Ollama error: {err}");
                                }
                                if let Some(msg) = event.message {
                                    thinking_chars += msg.thinking.len();
                                    accumulated.push_str(&msg.content);
                                }
                            }
                        }
                        break 'stream;
                    }
                    Err(e) => {
                        return Err(e).context("stream interrupted while reading response from Ollama");
                    }
                }
            }
            _ = ticker.tick() => {
                let idle_limit = if accumulated.is_empty() && thinking_chars == 0 {
                    Duration::from_secs(180)
                } else {
                    Duration::from_secs(60)
                };
                if last_activity.elapsed() >= idle_limit {
                    if accumulated.len() > 120 && accumulated.to_ascii_lowercase().contains("<body") {
                        break 'stream;
                    }
                    bail!(
                        "Stream stalled waiting for `{}` (no tokens for {}s). Try lowering Max image px in Settings or retrying.",
                        settings.model,
                        idle_limit.as_secs()
                    );
                }
                let phase = if !accumulated.is_empty() {
                    ChatPhase::Generating
                } else if thinking_chars > 0 {
                    ChatPhase::Thinking
                } else {
                    ChatPhase::Waiting
                };
                on_progress(ChatProgress {
                    phase,
                    chars: accumulated.len(),
                    thinking_chars,
                    elapsed_secs: started.elapsed().as_secs(),
                });
            }
        }
    }

    if accumulated.trim().is_empty() {
        if thinking_chars > 0 {
            bail!(
                "Model `{}` emitted only reasoning tokens and no HTML output. Switch to a direct vision/HTML model such as `qwen2.5vl:3b` in Settings.",
                settings.model
            );
        }
        return Err(anyhow!("the model returned an empty response"));
    }

    let html = clean_html(&accumulated);
    if loop_trimmed
        && (html.trim().len() < 60 || !html.to_ascii_lowercase().contains("<body"))
    {
        bail!(
            "Model `{}` got stuck in a repetition loop. Try lowering Max image px in Settings or retrying.",
            settings.model
        );
    }
    if html.trim().is_empty() {
        return Err(anyhow!("the model returned an empty response"));
    }
    Ok(html)
}

/// Send the prepared image to the model with live streaming progress and return a full HTML document.
pub async fn image_to_html_streaming<F>(
    settings: &Settings,
    image_bytes: &[u8],
    on_progress: F,
) -> Result<String>
where
    F: FnMut(ChatProgress),
{
    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    send_chat_streaming(settings, &settings.prompt, vec![encoded], on_progress).await
}

/// Generate a full HTML document directly from a user text prompt with live streaming progress.
pub async fn prompt_to_html_streaming<F>(
    settings: &Settings,
    user_request: &str,
    on_progress: F,
) -> Result<String>
where
    F: FnMut(ChatProgress),
{
    let prompt = format!("{DOCUMENT_GENERATION_PROMPT}\n\nUser request:\n{}", user_request.trim());
    send_chat_streaming(settings, &prompt, Vec::new(), on_progress).await
}

/// Build the follow-up refinement prompt with concise prior-turn context and the latest HTML.
pub fn build_refinement_prompt(
    current_html: &str,
    prior_requests: &[String],
    instruction: &str,
) -> String {
    let mut prompt = String::from(REFINE_HTML_PROMPT);
    if !prior_requests.is_empty() {
        prompt.push_str("\n\nPrevious refinement requests already applied:\n");
        for (idx, req) in prior_requests.iter().rev().take(6).collect::<Vec<_>>().into_iter().rev().enumerate() {
            prompt.push_str(&format!("{}. {}\n", idx + 1, req.trim()));
        }
    }
    prompt.push_str("\n\nCurrent HTML document:\n");
    prompt.push_str(current_html.trim());
    prompt.push_str("\n\nUser refinement request:\n");
    prompt.push_str(instruction.trim());
    prompt
}

/// Replace large inline `data:image/...;base64,...` URIs with compact placeholders
/// before sending HTML to Ollama so base64 payloads do not overflow `num_ctx` or get corrupted.
pub fn protect_data_urls(html: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(html.len());
    let mut saved = Vec::new();
    let mut rest = html;

    while let Some(pos) = rest.find("data:image/") {
        out.push_str(&rest[..pos]);
        let candidate = &rest[pos..];
        let end = candidate
            .find(['"', '\'', ')', '>', '<', ' ', '\n', '\r', '\t'])
            .unwrap_or(candidate.len());
        let uri = &candidate[..end];
        if uri.contains(";base64,") && uri.len() > 256 {
            let idx = saved.len();
            saved.push(uri.to_string());
            out.push_str(&format!("__DF_DATA_IMG_{idx}__"));
        } else {
            out.push_str(uri);
        }
        rest = &candidate[end..];
    }
    out.push_str(rest);
    (out, saved)
}

pub fn restore_data_urls(html: &str, saved: &[String]) -> String {
    let mut out = html.to_string();
    for (idx, uri) in saved.iter().enumerate() {
        let token = format!("__DF_DATA_IMG_{idx}__");
        out = out.replace(&token, uri);
    }
    out
}

/// Iteratively refine an existing HTML document with live streaming progress.
pub async fn refine_html_streaming<F>(
    settings: &Settings,
    current_html: &str,
    prior_requests: &[String],
    instruction: &str,
    on_progress: F,
) -> Result<String>
where
    F: FnMut(ChatProgress),
{
    if instruction.trim().is_empty() {
        bail!("refinement instruction cannot be empty");
    }
    if current_html.trim().is_empty() {
        bail!("no HTML document available to refine");
    }
    let (compact_html, saved_urls) = protect_data_urls(current_html);
    let prompt = build_refinement_prompt(&compact_html, prior_requests, instruction);
    let refined = send_chat_streaming(settings, &prompt, Vec::new(), on_progress).await?;
    Ok(restore_data_urls(&refined, &saved_urls))
}

/// List models installed in Ollama (used by the settings page).
pub async fn list_models(base_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let tags: TagsResponse = client(Duration::from_secs(5))?
        .get(&url)
        .send()
        .await
        .with_context(|| format!("could not reach Ollama at {url}. Is `ollama serve` running?"))?
        .error_for_status()?
        .json()
        .await?;
    Ok(tags.models.into_iter().map(|m| m.name).collect())
}

/// Check whether `model` appears in `installed` (matching implicit `:latest` tags).
pub fn model_in_list(installed: &[String], model: &str) -> bool {
    let target = model.trim();
    if target.is_empty() {
        return false;
    }
    let with_latest = if target.contains(':') { None } else { Some(format!("{target}:latest")) };
    installed.iter().any(|m| {
        m.eq_ignore_ascii_case(target)
            || with_latest.as_deref().is_some_and(|wl| m.eq_ignore_ascii_case(wl))
    })
}

/// Check with the local Ollama server if `model` is already downloaded.
pub async fn is_model_installed(base_url: &str, model: &str) -> Result<bool> {
    let models = list_models(base_url).await?;
    Ok(model_in_list(&models, model))
}

/// Download (pull) `model` in Ollama via `POST /api/pull`, streaming progress updates.
pub async fn pull_model<F>(base_url: &str, model: &str, mut on_progress: F) -> Result<()>
where
    F: FnMut(PullUpdate),
{
    let model = model.trim();
    if model.is_empty() {
        bail!("model name cannot be empty");
    }

    let url = format!("{}/api/pull", base_url.trim_end_matches('/'));
    let mut response = client(Duration::from_secs(60 * 60))?
        .post(&url)
        .json(&serde_json::json!({ "model": model, "stream": true }))
        .send()
        .await
        .with_context(|| format!("could not reach Ollama at {url}. Is `ollama serve` running?"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Ollama pull failed ({status}): {body}");
    }

    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await.context("stream interrupted while pulling model")? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer.drain(..=pos);
            if line.is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<PullStreamLine>(&line) {
                if let Some(err) = event.error {
                    bail!("Failed to pull `{model}`: {err}");
                }
                let update = match (event.completed, event.total) {
                    (Some(c), Some(t)) if t > 0 => {
                        let pct = ((c as f64 / t as f64) * 100.0).round().clamp(0.0, 100.0) as u32;
                        let mb_c = c as f64 / (1024.0 * 1024.0);
                        let mb_t = t as f64 / (1024.0 * 1024.0);
                        PullUpdate {
                            text: format!(
                                "Downloading {model}: {} ({pct}% — {mb_c:.0} / {mb_t:.0} MB)",
                                event.status
                            ),
                            percent: Some(pct),
                        }
                    }
                    _ => PullUpdate {
                        text: format!("Downloading {model}: {}", event.status),
                        percent: None,
                    },
                };
                on_progress(update);
            }
        }
    }

    Ok(())
}

/// Strip Markdown code fences / chatter around the HTML and make sure we
/// end up with a complete document.
pub fn clean_html(raw: &str) -> String {
    let mut s = raw.trim();

    // Remove ```html ... ``` fences if the model added them.
    if let Some(start) = s.find("```") {
        let after = &s[start + 3..];
        let after = after.strip_prefix("html").unwrap_or(after);
        s = match after.rfind("```") {
            Some(end) => &after[..end],
            None => after,
        };
        s = s.trim();
    }

    // Cut any text before the document starts / after it ends, and close truncated docs.
    let lower = s.to_ascii_lowercase();
    if let Some(start) = lower.find("<!doctype").or_else(|| lower.find("<html")) {
        if let Some(end) = lower.rfind("</html>") {
            return s[start..end + "</html>".len()].to_string();
        }
        let mut partial = s[start..].trim_end().to_string();
        if !lower[start..].contains("</body>") {
            partial.push_str("\n</body>");
        }
        partial.push_str("\n</html>");
        return partial;
    }

    // Fragment only: wrap it in a printable document.
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  @page {{ size: A4; margin: 15mm; }}
  body {{ font-family: serif; font-size: 11pt; line-height: 1.4; }}
  table {{ border-collapse: collapse; width: 100%; }}
  td, th {{ border: 1px solid #000; padding: 4px; }}
</style>
</head>
<body>
{s}
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_html_strips_markdown_fences_and_chatter() {
        let raw = "Sure! Here is the document:\n```html\n<!DOCTYPE html><html><body>Hi</body></html>\n```\nHope this helps.";
        assert_eq!(clean_html(raw), "<!DOCTYPE html><html><body>Hi</body></html>");
    }

    #[test]
    fn build_refinement_prompt_includes_prior_turns_and_current_html() {
        let prompt = build_refinement_prompt(
            "<!DOCTYPE html><html><body><h1>Invoice</h1></body></html>",
            &["Make header green".into()],
            "Add a 10% tax row",
        );
        assert!(prompt.contains("1. Make header green"));
        assert!(prompt.contains("<h1>Invoice</h1>"));
        assert!(prompt.contains("Add a 10% tax row"));
    }

    #[test]
    fn model_in_list_matches_implicit_latest_tag() {
        let list = vec!["qwen2.5vl:3b".into(), "llama3:latest".into()];
        assert!(model_in_list(&list, "qwen2.5vl:3b"));
        assert!(model_in_list(&list, "llama3"));
        assert!(!model_in_list(&list, "gemma3"));
    }

    #[test]
    fn protect_and_restore_data_urls_preserves_embedded_images() {
        let big_b64 = "A".repeat(400);
        let html = format!(
            "<!DOCTYPE html><html><body><img src=\"data:image/png;base64,{big_b64}\" alt=\"Logo\"><p>Hi</p></body></html>"
        );
        let (compact, saved) = protect_data_urls(&html);
        assert_eq!(saved.len(), 1);
        assert!(compact.contains("__DF_DATA_IMG_0__"));
        assert!(!compact.contains(&big_b64));
        assert_eq!(restore_data_urls(&compact, &saved), html);
    }

    #[test]
    fn detect_repetition_loop_catches_repeated_rows_and_ignores_normal_html() {
        let normal = "<!DOCTYPE html><html><body><h1>Invoice</h1><table><tr><td>Item 1</td><td>$10</td></tr><tr><td>Item 2</td><td>$20</td></tr></table></body></html>";
        assert_eq!(detect_repetition_loop(normal), None);

        let repeated_row = "<tr><td></td><td></td></tr>";
        let looped = format!("<!DOCTYPE html><html><body><table>{}", repeated_row.repeat(14));
        let cut = detect_repetition_loop(&looped).expect("should detect repeating row loop");
        let truncated = &looped[..cut];
        assert!(truncated.contains("<table>"));
        assert!(truncated.len() < looped.len());

        let blank_run = format!("<!DOCTYPE html><html><body><p>Hi</p>{}", " ".repeat(260));
        let cut_ws = detect_repetition_loop(&blank_run).expect("should detect repeating space run");
        assert!(cut_ws < blank_run.len());
    }

    #[test]
    fn clean_html_closes_truncated_document() {
        let truncated = "<!DOCTYPE html><html><body><h1>Draft</h1><p>Hello";
        let cleaned = clean_html(truncated);
        assert!(cleaned.ends_with("</body>\n</html>"));

        let missing_html_close = "<!DOCTYPE html><html><body><h1>Draft</h1></body>";
        let cleaned2 = clean_html(missing_html_close);
        assert!(cleaned2.ends_with("</body>\n</html>"));
        assert_eq!(cleaned2.matches("</body>").count(), 1);
    }

    #[test]
    fn prepare_image_resizes_and_encodes_png() {
        let img = image::RgbImage::from_fn(1200, 600, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 128])
        });
        let mut png_in = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut png_in, image::ImageFormat::Png)
            .unwrap();

        let prepared = prepare_image(&png_in.into_inner(), 512).unwrap();
        assert!(prepared.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
        let decoded = image::load_from_memory(&prepared).unwrap();
        assert_eq!(decoded.width(), 512);
        assert_eq!(decoded.height(), 256);
    }
}
