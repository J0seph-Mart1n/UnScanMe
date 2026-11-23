//! Visual editor protocol and embedded JS canvas engine.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Embedded visual canvas editor script injected via `document::eval`.
pub const CANVAS_EDITOR_JS: &str = include_str!("../assets/editor/canvas_editor.js");

/// Shapes supported by the visual editor's Shapes menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Line,
    Arrow,
}

/// Inline rich-text formatting commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCommand {
    Bold,
    Italic,
    Underline,
    AlignLeft,
    AlignCenter,
    AlignRight,
}

/// Stacking order operations for the selected element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerDirection {
    Forward,
    Backward,
}

/// Interactive table structure operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableActionKind {
    InsertRowAbove,
    InsertRowBelow,
    DeleteRow,
    InsertColLeft,
    InsertColRight,
    DeleteCol,
    ToggleHeader,
    SelectTable,
}

/// Commands sent from Rust (`VisualEditor`) into the JS canvas controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorCommand {
    LoadHtml { html: String },
    InsertText,
    InsertShape { shape: ShapeKind },
    InsertImage { data_url: String, alt: String },
    InsertTable { rows: u32, cols: u32 },
    TableAction { action: TableActionKind },
    SetStyle { prop: String, value: String },
    ToggleFormat { cmd: FormatCommand },
    DeleteSelected,
    DuplicateSelected,
    Reorder { direction: LayerDirection },
    Undo,
    Redo,
    FlushHtml,
    Shutdown,
}

/// Snapshot of the currently selected element's geometry and styles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionInfo {
    pub label: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub font_size: u32,
    pub color: String,
    pub background: String,
    pub border_color: String,
    pub border_width: u32,
    pub border_radius: u32,
    pub opacity: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub text_align: String,
    pub is_text_editable: bool,
    #[serde(default)]
    pub in_table: bool,
    #[serde(default)]
    pub table_rows: u32,
    #[serde(default)]
    pub table_cols: u32,
    #[serde(default)]
    pub cell_row: u32,
    #[serde(default)]
    pub cell_col: u32,
    #[serde(default)]
    pub is_header_cell: bool,
}

/// Events emitted from the JS canvas controller back to Rust.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorEvent {
    Ready,
    HtmlChanged {
        html: String,
        can_undo: bool,
        can_redo: bool,
    },
    SelectionChanged {
        info: Option<SelectionInfo>,
        can_undo: bool,
        can_redo: bool,
    },
}

/// Infer an image MIME type from a file extension for `data:` preview/embed URLs.
pub fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("tif" | "tiff") => "image/tiff",
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_command_round_trip() {
        let cmds = vec![
            EditorCommand::LoadHtml { html: "<html></html>".into() },
            EditorCommand::InsertShape { shape: ShapeKind::Arrow },
            EditorCommand::InsertImage {
                data_url: "data:image/png;base64,AAAA".into(),
                alt: "logo".into(),
            },
            EditorCommand::InsertTable { rows: 3, cols: 4 },
            EditorCommand::TableAction { action: TableActionKind::InsertRowBelow },
            EditorCommand::TableAction { action: TableActionKind::ToggleHeader },
            EditorCommand::ToggleFormat { cmd: FormatCommand::Bold },
            EditorCommand::Reorder { direction: LayerDirection::Forward },
            EditorCommand::Shutdown,
        ];
        for cmd in cmds {
            let json = serde_json::to_string(&cmd).unwrap();
            let decoded: EditorCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, decoded);
        }
    }

    #[test]
    fn editor_event_round_trip() {
        let evt = EditorEvent::SelectionChanged {
            info: Some(SelectionInfo {
                label: "Text".into(),
                kind: "text".into(),
                x: 40,
                y: 80,
                width: 240,
                height: 44,
                font_size: 16,
                color: "#111827".into(),
                background: "transparent".into(),
                border_color: "#3b82f6".into(),
                border_width: 0,
                border_radius: 4,
                opacity: 100,
                bold: true,
                italic: false,
                underline: false,
                text_align: "left".into(),
                is_text_editable: true,
                in_table: true,
                table_rows: 3,
                table_cols: 4,
                cell_row: 2,
                cell_col: 1,
                is_header_cell: false,
            }),
            can_undo: true,
            can_redo: false,
        };
        let json = serde_json::to_string(&evt).unwrap();
        let decoded: EditorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(evt, decoded);
    }

    #[test]
    fn mime_detection_handles_common_extensions() {
        assert_eq!(mime_for(Path::new("scan.JPG")), "image/jpeg");
        assert_eq!(mime_for(Path::new("logo.svg")), "image/svg+xml");
        assert_eq!(mime_for(Path::new("page.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("unknown.bin")), "image/png");
    }
}
