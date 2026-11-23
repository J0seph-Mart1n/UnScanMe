//! PDF / Print page setup model and idempotent CSS `@page` injection.

use serde::{Deserialize, Serialize};

pub const PAGE_SETUP_STYLE_ID: &str = "__df_page_setup__";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PaperSize {
    #[default]
    A4,
    Letter,
    Legal,
    A3,
    A5,
}

impl PaperSize {
    pub const ALL: [PaperSize; 5] = [
        PaperSize::A4,
        PaperSize::Letter,
        PaperSize::Legal,
        PaperSize::A3,
        PaperSize::A5,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::Letter => "Letter",
            PaperSize::Legal => "Legal",
            PaperSize::A3 => "A3",
            PaperSize::A5 => "A5",
        }
    }

    pub fn css_token(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::Letter => "letter",
            PaperSize::Legal => "legal",
            PaperSize::A3 => "A3",
            PaperSize::A5 => "A5",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

impl Orientation {
    pub fn label(self) -> &'static str {
        match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
        }
    }

    pub fn css_token(self) -> &'static str {
        match self {
            Orientation::Portrait => "portrait",
            Orientation::Landscape => "landscape",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSetup {
    pub paper: PaperSize,
    pub orientation: Orientation,
    pub margin_mm: u32,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            orientation: Orientation::Portrait,
            margin_mm: 15,
        }
    }
}

impl PageSetup {
    pub fn summary(&self) -> String {
        format!(
            "{} · {} · {}mm",
            self.paper.label(),
            if self.orientation == Orientation::Portrait { "Port." } else { "Land." },
            self.margin_mm
        )
    }

    pub fn to_style_tag(self) -> String {
        let size = self.paper.css_token();
        let orient = self.orientation.css_token();
        let margin = self.margin_mm.min(60);
        format!(
            r#"<style id="{PAGE_SETUP_STYLE_ID}">@page {{ size: {size} {orient}; margin: {margin}mm; }}</style>"#
        )
    }
}

/// Remove any previously injected `__df_page_setup__` style tag and inject the
/// current `@page` rule at the end of `<head>` (or start of document) so it
/// takes CSS cascade precedence during preview and headless Chromium PDF export.
pub fn apply_page_setup(html: &str, setup: &PageSetup) -> String {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let cleaned = strip_existing_page_setup(trimmed);
    let style_tag = setup.to_style_tag();
    let lower = cleaned.to_ascii_lowercase();

    if let Some(head_close) = lower.rfind("</head>") {
        let mut out = String::with_capacity(cleaned.len() + style_tag.len() + 2);
        out.push_str(&cleaned[..head_close]);
        if !cleaned[..head_close].ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&style_tag);
        out.push('\n');
        out.push_str(&cleaned[head_close..]);
        return out;
    }

    if let Some(body_open) = lower.find("<body") {
        let mut out = String::with_capacity(cleaned.len() + style_tag.len() + 16);
        out.push_str(&cleaned[..body_open]);
        out.push_str("<head>");
        out.push_str(&style_tag);
        out.push_str("</head>\n");
        out.push_str(&cleaned[body_open..]);
        return out;
    }

    format!("{style_tag}\n{cleaned}")
}

fn strip_existing_page_setup(html: &str) -> String {
    let marker = format!(r#"id="{PAGE_SETUP_STYLE_ID}""#);
    let mut current = html.to_string();

    while let Some(id_pos) = current.find(&marker) {
        let Some(style_start) = current[..id_pos].rfind("<style") else {
            break;
        };
        let Some(rel_end) = current[id_pos..].find("</style>") else {
            break;
        };
        let style_end = id_pos + rel_end + "</style>".len();
        let before = current[..style_start].trim_end_matches([' ', '\t']);
        let after = current[style_end..].trim_start_matches(['\r', '\n']);
        if before.ends_with('\n') || after.is_empty() {
            current = format!("{before}{after}");
        } else {
            current = format!("{before}\n{after}");
        }
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_html_returns_empty() {
        assert_eq!(apply_page_setup("   ", &PageSetup::default()), "");
    }

    #[test]
    fn injects_before_closing_head_and_is_idempotent() {
        let input = "<!DOCTYPE html><html><head><title>Test</title></head><body><p>Hi</p></body></html>";
        let setup_a = PageSetup {
            paper: PaperSize::Letter,
            orientation: Orientation::Landscape,
            margin_mm: 10,
        };
        let once = apply_page_setup(input, &setup_a);
        assert!(once.contains("@page { size: letter landscape; margin: 10mm; }"));
        assert_eq!(once.matches(PAGE_SETUP_STYLE_ID).count(), 1);

        let setup_b = PageSetup {
            paper: PaperSize::Legal,
            orientation: Orientation::Portrait,
            margin_mm: 25,
        };
        let twice = apply_page_setup(&once, &setup_b);
        assert!(twice.contains("@page { size: legal portrait; margin: 25mm; }"));
        assert!(!twice.contains("letter landscape"));
        assert_eq!(twice.matches(PAGE_SETUP_STYLE_ID).count(), 1);
    }

    #[test]
    fn injects_when_head_is_missing() {
        let input = "<html><body><h1>Doc</h1></body></html>";
        let out = apply_page_setup(input, &PageSetup::default());
        assert!(out.contains("<head><style id=\"__df_page_setup__\">"));
        assert!(out.contains("@page { size: A4 portrait; margin: 15mm; }"));
    }

    #[test]
    fn clamps_excessive_margin() {
        let setup = PageSetup {
            paper: PaperSize::A5,
            orientation: Orientation::Portrait,
            margin_mm: 200,
        };
        assert!(setup.to_style_tag().contains("margin: 60mm;"));
    }
}
