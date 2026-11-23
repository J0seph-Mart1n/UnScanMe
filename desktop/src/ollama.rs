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
struct ChatResponse {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
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
2. **Layout & Print Styling**: Use clean modern HTML5 and CSS inside a single <style> block in <head>. Include `@page { size: A4; margin: 15mm; }` and ensure the layout fits cleanly on an A4 page.
3. **Tables & Formatting**: Style tables with `border-collapse: collapse`, clear borders, readable padding, and right-aligned numbers/currency.
4. **No External Resources**: Do NOT use <img> tags, external URLs, JavaScript, or external fonts. Use inline SVG or CSS shapes only if a simple logo/divider is helpful.
5. **Output Format**: Return ONLY the raw HTML code starting with <!DOCTYPE html> and ending with </html>. Do NOT wrap the output in markdown code fences or include explanatory text."#;

async fn send_chat(settings: &Settings, prompt: &str, images: Vec<String>) -> Result<String> {
    let url = format!("{}/api/chat", settings.ollama_url.trim_end_matches('/'));
    let request = ChatRequest {
        model: &settings.model,
        messages: vec![ChatMessage { role: "user", content: prompt, images }],
        stream: false,
        options: ChatOptions { temperature: settings.temperature, num_ctx: settings.num_ctx },
    };

    // Local inference of a full page can take minutes on small GPUs.
    let response = client(Duration::from_secs(30 * 60))?
        .post(&url)
        .json(&request)
        .send()
        .await
        .with_context(|| format!("could not reach Ollama at {url}. Is `ollama serve` running?"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Ollama returned {status}: {body}");
    }

    let chat: ChatResponse = response.json().await.context("invalid response from Ollama")?;
    let html = clean_html(&chat.message.content);
    if html.trim().is_empty() {
        return Err(anyhow!("the model returned an empty response"));
    }
    Ok(html)
}

/// Send the image to the model and return a full HTML document.
pub async fn image_to_html(settings: &Settings, png: &[u8]) -> Result<String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    send_chat(settings, &settings.prompt, vec![encoded]).await
}

/// Generate a full HTML document directly from a user text prompt (no image).
pub async fn prompt_to_html(settings: &Settings, user_request: &str) -> Result<String> {
    let prompt = format!("{DOCUMENT_GENERATION_PROMPT}\n\nUser request:\n{}", user_request.trim());
    send_chat(settings, &prompt, Vec::new()).await
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

    // Cut any text before the document starts / after it ends.
    let lower = s.to_ascii_lowercase();
    if let Some(start) = lower.find("<!doctype").or_else(|| lower.find("<html")) {
        let end = lower.rfind("</html>").map(|e| e + "</html>".len()).unwrap_or(s.len());
        return s[start..end].to_string();
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
