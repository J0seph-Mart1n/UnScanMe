//! Persistent application settings, stored as JSON in `~/.config/unscanme/settings.json`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LEGACY_DEFAULT_PROMPT: &str = r#"You are an expert image-to-HTML converter agent.
Your task is to analyze the provided image and generate a SINGLE, self-contained HTML file
that faithfully reproduces everything visible in the image.

Follow these rules strictly:

1. **Text Extraction**: Extract ALL text from the image exactly as it appears — every heading,
   paragraph, label, caption, watermark, button text, or any other visible text. Preserve the
   original wording, capitalisation, and punctuation.

2. **Layout & Structure**: Reproduce the spatial layout of the image using modern HTML5 and CSS.
   Use flexbox or CSS grid where appropriate. If the image shows a multi-column layout, cards,
   a navigation bar, a footer, or any other structural element, replicate that structure
   precisely in HTML.

3. **Styling & Design**: Match the visual design as closely as possible:
   - Use the exact (or closest possible) colours for backgrounds, text, borders, and accents.
   - Match font sizes, weights, and styles (bold, italic, etc.).
   - Reproduce rounded corners, shadows, gradients, and spacing.
   - If the image uses a dark or light theme, replicate that theme.

4. **Images & Icons**: If the image contains photographs, illustrations, icons, or logos,
   do NOT use <img> tags or external placeholder URLs under any circumstances, as they cause rendering hangs.
   Instead, replicate them using purely CSS styling (e.g., background colours/shapes), inline SVG code, or Unicode symbols.

5. **Responsiveness**: Make the HTML reasonably responsive so it looks good at different
   viewport widths, but prioritise accuracy over responsiveness.

6. **Output Format**: Return ONLY the raw HTML code starting with <!DOCTYPE html> and ending
   with </html>. Do NOT wrap the output in markdown code fences. Do NOT include any
   explanatory text before or after the HTML."#;

const INTERMEDIATE_DEFAULT_PROMPT: &str = r#"You are an expert document and image-to-HTML5 converter agent.
Your task is to analyze the provided image (a scanned document, invoice, form, receipt, screenshot,
cropped extract, or rasterized PDF page) and generate a SINGLE, complete, self-contained HTML5
document that faithfully reproduces all visible content and structure.

Follow these rules strictly:

1. **Complete Text & Data Extraction**: Extract ALL visible text verbatim — every heading,
   paragraph, label, date, reference number, line item, quantity, price, subtotal, tax, total,
   footnote, caption, or badge. Preserve original wording, capitalisation, numbers, and punctuation.

2. **Visual-Editor-Ready Semantic Structure**: Build the document with clean, discrete semantic HTML5
   elements (`<header>`, `<section>`, `<h1>`–`<h3>`, `<p>`, `<ul>`/`<ol>`, `<div>`, `<table>`)
   and put all CSS inside a single `<style>` block in `<head>`. Avoid deeply nested wrappers,
   pseudo-element-only text, or rigid fixed heights on text containers so individual blocks can be
   clicked, inline-edited, moved, resized, and refined cleanly in a visual editor or AI follow-up chat.

3. **Interactive Table Editor Compatibility**: For ANY tabular data, line items, pricing breakdowns,
   schedules, or key-value grids, ALWAYS use real semantic `<table>` markup (`<thead>`, `<tbody>`,
   `<tr>`, `<th>`, `<td>`) rather than Flexbox/Grid `<div>` rows. Style tables with
   `border-collapse: collapse`, clear cell borders and padding, and right-aligned numeric/currency cells.

4. **Page Setup & Print Styling**: Set `*, *::before, *::after { box-sizing: border-box; }` and
   include a clean `@page { size: A4 portrait; margin: 15mm; }` rule in `<style>` so the layout
   fits cleanly on standard paper and adapts automatically when custom Page Setup paper sizes,
   orientations, or margins are applied.

5. **Visual Design & Never Wrap the Page as an Image**:
   - Match the original colours, typography weights/sizes, borders, rounded corners, and spacing.
   - Do NOT use `<img>` tags (neither external URLs nor base64 `data:image` URIs), external fonts, or `<script>` tags. NEVER wrap the scanned page or document as an image; reconstruct every visible text element, heading, label, and table as individually editable HTML5 elements.
   - Replicate logos, stamps, seals, dividers, or simple diagrams using inline `<svg>`, CSS shapes, or Unicode symbols.

6. **Output Format**: Return ONLY the raw HTML5 code starting with `<!DOCTYPE html>` and ending
   with `</html>`. Do NOT wrap the output in markdown code fences or add any explanatory text."#;

pub const DEFAULT_PROMPT: &str = r#"You are an expert document and image-to-HTML5 converter agent.
Your task is to analyze the provided image (a scanned document, invoice, form, receipt, screenshot,
cropped extract, or rasterized PDF page) and generate a SINGLE, complete, self-contained HTML5
document that faithfully reproduces all visible content and structure.

Follow these rules strictly:

1. **Complete Text & Data Extraction**: Extract ALL visible text verbatim — every heading,
   paragraph, label, date, reference number, line item, quantity, price, subtotal, tax, total,
   footnote, caption, or badge. Preserve original wording, capitalisation, numbers, and punctuation.

2. **Visual-Editor-Ready Semantic Structure**: Build the document with clean, discrete semantic HTML5
   elements (`<header>`, `<section>`, `<h1>`–`<h3>`, `<p>`, `<ul>`/`<ol>`, `<div>`, `<table>`)
   and put all CSS inside a single `<style>` block in `<head>`. Avoid deeply nested wrappers,
   pseudo-element-only text, or rigid fixed heights on text containers so individual blocks can be
   clicked, inline-edited, moved, resized, and refined cleanly in a visual editor or AI follow-up chat.

3. **Interactive Table Editor Compatibility**: For ANY tabular data, line items, pricing breakdowns,
   schedules, or key-value grids, ALWAYS use real semantic `<table>` markup (`<thead>`, `<tbody>`,
   `<tr>`, `<th>`, `<td>`) rather than Flexbox/Grid `<div>` rows. Style tables with
   `border-collapse: collapse`, clear cell borders and padding, and right-aligned numeric/currency cells.

4. **Page Setup & Print Styling**: Set `*, *::before, *::after { box-sizing: border-box; }` and
   include a clean `@page { size: A4 portrait; margin: 15mm; }` rule in `<style>` so the layout
   fits cleanly on standard paper and adapts automatically when custom Page Setup paper sizes,
   orientations, or margins are applied.

5. **Visual Design & Self-Contained Graphics**:
   - Match the original colours, typography weights/sizes, borders, rounded corners, and spacing.
   - Do NOT use external `<img>` URLs, external fonts, or `<script>` tags (they cause rendering hangs).
   - Replicate logos, stamps, seals, dividers, or simple diagrams using inline `<svg>`, CSS shapes, or Unicode symbols.

6. **Output Format**: Return ONLY the raw HTML5 code starting with `<!DOCTYPE html>` and ending
   with `</html>`. Do NOT wrap the output in markdown code fences or add any explanatory text."#;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Base URL of the local Ollama server.
    pub ollama_url: String,
    /// Ollama model tag of a vision-capable model, e.g. `qwen2.5vl:3b`.
    pub model: String,
    /// Instruction prompt sent with the image.
    pub prompt: String,
    /// Model context window (tokens). Larger = longer documents, more VRAM.
    pub num_ctx: u32,
    /// Sampling temperature. Keep low for faithful transcription.
    pub temperature: f32,
    /// Longest image side (pixels) sent to the model. Bigger images cost more VRAM / time.
    pub max_image_dim: u32,
    /// Chromium / Chrome executable used for HTML -> PDF.
    pub chromium_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            model: "qwen2.5vl:3b".into(),
            prompt: DEFAULT_PROMPT.into(),
            num_ctx: 8192,
            temperature: 0.1,
            max_image_dim: 1536,
            chromium_path: crate::pdf::detect_chromium(),
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("unscanme").join("settings.json"))
}

fn legacy_settings_paths() -> Vec<PathBuf> {
    match dirs::config_dir() {
        Some(d) => vec![
            d.join("i-hate-scans").join("settings.json"),
            d.join("img_to_pdf").join("settings.json"),
        ],
        None => Vec::new(),
    }
}

impl Settings {
    /// Upgrade an unmodified legacy default prompt (or empty prompt) to the current `DEFAULT_PROMPT`.
    pub fn migrate_legacy_prompt(&mut self) {
        let trimmed = self.prompt.trim();
        if trimmed.is_empty()
            || trimmed == LEGACY_DEFAULT_PROMPT.trim()
            || trimmed == INTERMEDIATE_DEFAULT_PROMPT.trim()
        {
            self.prompt = DEFAULT_PROMPT.into();
        }
    }

    /// Load settings from disk, falling back to legacy config paths or defaults on any error.
    pub fn load() -> Self {
        let mut settings: Self = settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .or_else(|| {
                legacy_settings_paths()
                    .into_iter()
                    .find_map(|p| std::fs::read_to_string(p).ok())
            })
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        settings.migrate_legacy_prompt();
        settings
    }

    pub fn save(&self) -> anyhow::Result<PathBuf> {
        let path = settings_path().ok_or_else(|| anyhow::anyhow!("no config directory found"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prompt_covers_new_editor_table_and_page_setup_rules() {
        assert!(DEFAULT_PROMPT.contains("<table>"));
        assert!(DEFAULT_PROMPT.contains("<thead>"));
        assert!(DEFAULT_PROMPT.contains("@page"));
        assert!(DEFAULT_PROMPT.contains("visual editor"));
    }

    #[test]
    fn migrate_legacy_prompt_upgrades_old_default_and_keeps_custom_prompt() {
        let mut legacy = Settings { prompt: LEGACY_DEFAULT_PROMPT.into(), ..Default::default() };
        legacy.migrate_legacy_prompt();
        assert_eq!(legacy.prompt, DEFAULT_PROMPT);

        let mut intermediate = Settings { prompt: INTERMEDIATE_DEFAULT_PROMPT.into(), ..Default::default() };
        intermediate.migrate_legacy_prompt();
        assert_eq!(intermediate.prompt, DEFAULT_PROMPT);

        let custom_text = "My custom invoice extraction prompt";
        let mut custom = Settings { prompt: custom_text.into(), ..Default::default() };
        custom.migrate_legacy_prompt();
        assert_eq!(custom.prompt, custom_text);
    }
}
