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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    #[default]
    Ollama,
    Gemini,
    OpenAi,
    Anthropic,
    OpenAiCompatible,
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl AiProvider {
    pub const CLOUD: [Self; 4] = [
        Self::Gemini,
        Self::OpenAi,
        Self::Anthropic,
        Self::OpenAiCompatible,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama (Local)",
            Self::Gemini => "Google Gemini",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic Claude",
            Self::OpenAiCompatible => "OpenAI-Compatible",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Ollama => "Local",
            Self::Gemini => "Gemini",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Claude",
            Self::OpenAiCompatible => "Custom API",
        }
    }

    pub fn is_cloud(self) -> bool {
        !matches!(self, Self::Ollama)
    }

    pub fn requires_api_key(self) -> bool {
        matches!(self, Self::Gemini | Self::OpenAi | Self::Anthropic)
    }

    pub fn env_var_name(self) -> Option<&'static str> {
        match self {
            Self::Gemini => Some("GEMINI_API_KEY"),
            Self::OpenAi => Some("OPENAI_API_KEY"),
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::Ollama | Self::OpenAiCompatible => None,
        }
    }

    pub fn suggested_models(self) -> &'static [&'static str] {
        match self {
            Self::Ollama => &[
                "qwen2.5vl:3b",
                "qwen2.5vl:7b",
                "llama3.2-vision:11b",
                "gemma3:4b",
            ],
            Self::Gemini => &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"],
            Self::OpenAi => &["gpt-4o-mini", "gpt-4o", "gpt-4.1-mini", "gpt-4.1"],
            Self::Anthropic => &[
                "claude-sonnet-4-20250514",
                "claude-3-7-sonnet-latest",
                "claude-3-5-haiku-latest",
            ],
            Self::OpenAiCompatible => &[
                "google/gemini-2.5-flash",
                "openai/gpt-4o-mini",
                "anthropic/claude-sonnet-4",
                "meta-llama/llama-4-scout-17b-16e-instruct",
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Active AI execution backend (local Ollama or a cloud API provider).
    pub provider: AiProvider,
    /// Last-used cloud provider restored when toggling between Local Ollama and Cloud API mode.
    pub last_cloud_provider: AiProvider,
    /// Base URL of the local Ollama server.
    pub ollama_url: String,
    /// Ollama model tag of a vision-capable model, e.g. `qwen2.5vl:3b`.
    pub model: String,
    /// Google Gemini API key (or leave empty to read `GEMINI_API_KEY`).
    pub gemini_api_key: String,
    /// Google Gemini model ID.
    pub gemini_model: String,
    /// OpenAI API key (or leave empty to read `OPENAI_API_KEY`).
    pub openai_api_key: String,
    /// OpenAI vision model ID.
    pub openai_model: String,
    /// Anthropic API key (or leave empty to read `ANTHROPIC_API_KEY`).
    pub anthropic_api_key: String,
    /// Anthropic Claude model ID.
    pub anthropic_model: String,
    /// Base URL for an OpenAI-compatible provider (OpenRouter, Groq, Together, etc.).
    pub custom_api_url: String,
    /// Optional Bearer API key for the OpenAI-compatible provider.
    pub custom_api_key: String,
    /// Model ID for the OpenAI-compatible provider.
    pub custom_model: String,
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
            provider: AiProvider::Ollama,
            last_cloud_provider: AiProvider::Ollama,
            ollama_url: "http://localhost:11434".into(),
            model: "qwen2.5vl:3b".into(),
            gemini_api_key: String::new(),
            gemini_model: "gemini-2.5-flash".into(),
            openai_api_key: String::new(),
            openai_model: "gpt-4o-mini".into(),
            anthropic_api_key: String::new(),
            anthropic_model: "claude-sonnet-4-20250514".into(),
            custom_api_url: "https://openrouter.ai/api/v1".into(),
            custom_api_key: String::new(),
            custom_model: "google/gemini-2.5-flash".into(),
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
    /// Model name/ID currently selected for the active provider.
    pub fn active_model(&self) -> &str {
        match self.provider {
            AiProvider::Ollama => &self.model,
            AiProvider::Gemini => &self.gemini_model,
            AiProvider::OpenAi => &self.openai_model,
            AiProvider::Anthropic => &self.anthropic_model,
            AiProvider::OpenAiCompatible => &self.custom_model,
        }
    }

    pub fn active_model_mut(&mut self) -> &mut String {
        match self.provider {
            AiProvider::Ollama => &mut self.model,
            AiProvider::Gemini => &mut self.gemini_model,
            AiProvider::OpenAi => &mut self.openai_model,
            AiProvider::Anthropic => &mut self.anthropic_model,
            AiProvider::OpenAiCompatible => &mut self.custom_model,
        }
    }

    pub fn active_api_key_stored(&self) -> &str {
        match self.provider {
            AiProvider::Ollama => "",
            AiProvider::Gemini => &self.gemini_api_key,
            AiProvider::OpenAi => &self.openai_api_key,
            AiProvider::Anthropic => &self.anthropic_api_key,
            AiProvider::OpenAiCompatible => &self.custom_api_key,
        }
    }

    pub fn active_api_key_mut(&mut self) -> Option<&mut String> {
        match self.provider {
            AiProvider::Ollama => None,
            AiProvider::Gemini => Some(&mut self.gemini_api_key),
            AiProvider::OpenAi => Some(&mut self.openai_api_key),
            AiProvider::Anthropic => Some(&mut self.anthropic_api_key),
            AiProvider::OpenAiCompatible => Some(&mut self.custom_api_key),
        }
    }

    fn api_key_for(&self, provider: AiProvider) -> String {
        let stored = match provider {
            AiProvider::Ollama => "",
            AiProvider::Gemini => &self.gemini_api_key,
            AiProvider::OpenAi => &self.openai_api_key,
            AiProvider::Anthropic => &self.anthropic_api_key,
            AiProvider::OpenAiCompatible => &self.custom_api_key,
        }
        .trim();
        if !stored.is_empty() {
            return stored.to_string();
        }
        if let Some(env_name) = provider.env_var_name() {
            if let Ok(val) = std::env::var(env_name) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        String::new()
    }

    /// Resolve the active API key and whether it came from an environment variable.
    pub fn effective_api_key(&self) -> (String, bool) {
        let stored = self.active_api_key_stored().trim();
        if !stored.is_empty() {
            return (stored.to_string(), false);
        }
        if let Some(env_name) = self.provider.env_var_name() {
            if let Ok(val) = std::env::var(env_name) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return (trimmed.to_string(), true);
                }
            }
        }
        (String::new(), false)
    }

    /// Cloud provider to activate when switching to API-key execution mode.
    pub fn preferred_cloud_provider(&self) -> AiProvider {
        if self.provider.is_cloud() {
            return self.provider;
        }
        if self.last_cloud_provider.is_cloud() {
            return self.last_cloud_provider;
        }
        AiProvider::CLOUD
            .iter()
            .copied()
            .find(|&p| !self.api_key_for(p).is_empty())
            .unwrap_or(AiProvider::Gemini)
    }

    /// Select an AI provider while remembering the latest cloud provider.
    pub fn set_provider(&mut self, provider: AiProvider) {
        if self.provider.is_cloud() && !provider.is_cloud() {
            self.last_cloud_provider = self.provider;
        }
        self.provider = provider;
        if provider.is_cloud() {
            self.last_cloud_provider = provider;
        }
    }

    /// Toggle between Local Ollama (`use_cloud_api = false`) and Cloud API execution (`true`).
    pub fn switch_execution_mode(&mut self, use_cloud_api: bool) {
        if use_cloud_api {
            let target = self.preferred_cloud_provider();
            self.set_provider(target);
        } else {
            self.set_provider(AiProvider::Ollama);
        }
    }

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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
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

    #[test]
    fn legacy_json_deserializes_with_ollama_default_and_cloud_defaults() {
        let legacy_json = r#"{"ollama_url":"http://localhost:11434","model":"qwen2.5vl:7b","num_ctx":4096}"#;
        let loaded: Settings = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(loaded.provider, AiProvider::Ollama);
        assert_eq!(loaded.active_model(), "qwen2.5vl:7b");
        assert_eq!(loaded.gemini_model, "gemini-2.5-flash");
        assert_eq!(loaded.openai_model, "gpt-4o-mini");
        assert_eq!(loaded.anthropic_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn active_model_and_key_switch_by_provider() {
        let mut s = Settings {
            provider: AiProvider::Gemini,
            gemini_api_key: "  gm-test-123  ".into(),
            ..Default::default()
        };
        *s.active_model_mut() = "gemini-2.5-pro".into();
        assert_eq!(s.active_model(), "gemini-2.5-pro");
        assert_eq!(s.effective_api_key(), ("gm-test-123".to_string(), false));

        s.provider = AiProvider::OpenAi;
        s.openai_api_key = "sk-test".into();
        assert_eq!(s.active_model(), "gpt-4o-mini");
        assert_eq!(s.effective_api_key(), ("sk-test".to_string(), false));
    }

    #[test]
    fn switch_execution_mode_remembers_last_cloud_provider() {
        let mut s = Settings {
            openai_api_key: "sk-configured".into(),
            ..Default::default()
        };
        assert_eq!(s.provider, AiProvider::Ollama);
        // Should pick OpenAI since only OpenAI has a configured key.
        s.switch_execution_mode(true);
        assert_eq!(s.provider, AiProvider::OpenAi);

        // Explicit cloud switch updates remembered cloud provider.
        s.set_provider(AiProvider::Anthropic);
        assert_eq!(s.last_cloud_provider, AiProvider::Anthropic);

        // Toggle back to local Ollama and then back to API key mode.
        s.switch_execution_mode(false);
        assert_eq!(s.provider, AiProvider::Ollama);
        s.switch_execution_mode(true);
        assert_eq!(s.provider, AiProvider::Anthropic);
    }
}
