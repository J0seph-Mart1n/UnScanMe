//! Interactive visual HTML editor backed by `canvas_editor.js` inside an iframe.

use crate::{
    editor::{
        self, EditorCommand, EditorEvent, FormatCommand, LayerDirection, SelectionInfo, ShapeKind,
        TableActionKind,
    },
    views::convert::Status,
};
use base64::Engine;
use dioxus::prelude::*;

fn send_cmd(eval_handle: Signal<Option<document::Eval>>, cmd: EditorCommand) {
    if let Some(ev) = *eval_handle.peek() {
        let _ = ev.send(cmd);
    }
}

fn color_input_val(raw: &str, fallback: &str) -> String {
    if raw.starts_with('#') && raw.len() == 7 {
        raw.to_string()
    } else {
        fallback.to_string()
    }
}

#[component]
pub fn VisualEditor(
    mut html: Signal<String>,
    busy: bool,
    mut status: Signal<Status>,
) -> Element {
    let mut eval_handle = use_signal(|| None::<document::Eval>);
    let mut selection = use_signal(|| None::<SelectionInfo>);
    let mut can_undo = use_signal(|| false);
    let mut can_redo = use_signal(|| false);
    let mut shapes_open = use_signal(|| false);
    let mut table_menu_open = use_signal(|| false);
    let mut last_synced_html = use_signal(String::new);
    let mut ready = use_signal(|| false);

    // Mount the JS canvas controller once and listen for editor events.
    use_effect(move || {
        if eval_handle.peek().is_some() {
            return;
        }
        let script = format!(
            "{}\nawait window.__unscanmeEditor.start(dioxus);",
            editor::CANVAS_EDITOR_JS
        );
        let mut eval = document::eval(&script);
        eval_handle.set(Some(eval));

        spawn(async move {
            while let Ok(evt) = eval.recv::<EditorEvent>().await {
                match evt {
                    EditorEvent::Ready => {
                        let current = html.peek().clone();
                        last_synced_html.set(current.clone());
                        ready.set(true);
                        let _ = eval.send(EditorCommand::LoadHtml { html: current });
                    }
                    EditorEvent::HtmlChanged {
                        html: updated,
                        can_undo: u,
                        can_redo: r,
                    } => {
                        last_synced_html.set(updated.clone());
                        html.set(updated);
                        can_undo.set(u);
                        can_redo.set(r);
                    }
                    EditorEvent::SelectionChanged {
                        info,
                        can_undo: u,
                        can_redo: r,
                    } => {
                        selection.set(info);
                        can_undo.set(u);
                        can_redo.set(r);
                    }
                }
            }
        });
    });

    // Reload the canvas only when `html` changes externally (AI generation, HTML source edit, clear).
    use_effect(move || {
        let current = html();
        if !ready() {
            return;
        }
        if current != *last_synced_html.peek() {
            if let Some(ev) = *eval_handle.peek() {
                last_synced_html.set(current.clone());
                let _ = ev.send(EditorCommand::LoadHtml { html: current });
            }
        }
    });

    use_drop(move || {
        if let Some(ev) = *eval_handle.peek() {
            let _ = ev.send(EditorCommand::Shutdown);
        }
    });

    let insert_image = move |_| async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Insert image into document")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "svg"])
            .pick_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        let bytes = file.read().await;
        if bytes.is_empty() {
            status.set(Status::Error("Selected image file was empty.".into()));
            return;
        }
        let data_url = format!(
            "data:{};base64,{}",
            editor::mime_for(&path),
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        );
        let alt = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Image".into());
        send_cmd(eval_handle, EditorCommand::InsertImage { data_url, alt });
    };

    let sel = selection();

    rsx! {
        div { class: "visual-editor",
            // Top editor toolbar: insert elements, history, selection actions
            div { class: "editor-toolbar", role: "toolbar", aria_label: "Visual editor tools",
                button {
                    class: "btn-small",
                    disabled: busy,
                    title: "Add editable text box",
                    onclick: move |_| {
                        shapes_open.set(false);
                        table_menu_open.set(false);
                        send_cmd(eval_handle, EditorCommand::InsertText);
                    },
                    "T+ Text"
                }

                div { class: "shape-dropdown",
                    button {
                        class: "btn-small",
                        disabled: busy,
                        title: "Insert shape",
                        aria_haspopup: "menu",
                        aria_expanded: "{shapes_open()}",
                        onclick: move |_| {
                            table_menu_open.set(false);
                            shapes_open.toggle();
                        },
                        "Shapes ▾"
                    }
                    if shapes_open() && !busy {
                        div {
                            class: "mode-backdrop",
                            onclick: move |_| shapes_open.set(false),
                        }
                        div { class: "shape-menu", role: "menu", aria_label: "Insert shape",
                            button {
                                class: "mode-option",
                                role: "menuitem",
                                onclick: move |_| {
                                    shapes_open.set(false);
                                    send_cmd(eval_handle, EditorCommand::InsertShape { shape: ShapeKind::Rect });
                                },
                                "Rectangle"
                            }
                            button {
                                class: "mode-option",
                                role: "menuitem",
                                onclick: move |_| {
                                    shapes_open.set(false);
                                    send_cmd(eval_handle, EditorCommand::InsertShape { shape: ShapeKind::Ellipse });
                                },
                                "Circle / Ellipse"
                            }
                            button {
                                class: "mode-option",
                                role: "menuitem",
                                onclick: move |_| {
                                    shapes_open.set(false);
                                    send_cmd(eval_handle, EditorCommand::InsertShape { shape: ShapeKind::Line });
                                },
                                "Line / Divider"
                            }
                            button {
                                class: "mode-option",
                                role: "menuitem",
                                onclick: move |_| {
                                    shapes_open.set(false);
                                    send_cmd(eval_handle, EditorCommand::InsertShape { shape: ShapeKind::Arrow });
                                },
                                "Arrow"
                            }
                        }
                    }
                }

                button {
                    class: "btn-small",
                    disabled: busy,
                    title: "Insert image from disk",
                    onclick: insert_image,
                    "Image"
                }

                div { class: "shape-dropdown",
                    button {
                        class: "btn-small",
                        disabled: busy,
                        title: "Insert editable table",
                        aria_haspopup: "menu",
                        aria_expanded: "{table_menu_open()}",
                        onclick: move |_| {
                            shapes_open.set(false);
                            table_menu_open.toggle();
                        },
                        "Table ▾"
                    }
                    if table_menu_open() && !busy {
                        div {
                            class: "mode-backdrop",
                            onclick: move |_| table_menu_open.set(false),
                        }
                        div { class: "shape-menu", role: "menu", aria_label: "Insert table",
                            for (rows, cols, label) in [
                                (2u32, 2u32, "2 × 2 Table"),
                                (3, 3, "3 × 3 Table"),
                                (4, 4, "4 × 4 Table"),
                                (5, 3, "5 × 3 Invoice"),
                                (6, 4, "6 × 4 Grid"),
                            ] {
                                button {
                                    class: "mode-option",
                                    role: "menuitem",
                                    onclick: move |_| {
                                        table_menu_open.set(false);
                                        send_cmd(eval_handle, EditorCommand::InsertTable { rows, cols });
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }

                span { class: "editor-sep", aria_hidden: "true" }

                button {
                    class: "btn-small",
                    disabled: busy || !can_undo(),
                    title: "Undo (Ctrl+Z)",
                    onclick: move |_| send_cmd(eval_handle, EditorCommand::Undo),
                    "Undo"
                }
                button {
                    class: "btn-small",
                    disabled: busy || !can_redo(),
                    title: "Redo (Ctrl+Y)",
                    onclick: move |_| send_cmd(eval_handle, EditorCommand::Redo),
                    "Redo"
                }

                if sel.is_some() {
                    span { class: "editor-sep", aria_hidden: "true" }
                    button {
                        class: "btn-small",
                        disabled: busy,
                        title: "Duplicate (Ctrl+D)",
                        onclick: move |_| send_cmd(eval_handle, EditorCommand::DuplicateSelected),
                        "Duplicate"
                    }
                    button {
                        class: "btn-small",
                        disabled: busy,
                        title: "Bring forward",
                        onclick: move |_| send_cmd(eval_handle, EditorCommand::Reorder { direction: LayerDirection::Forward }),
                        "Front"
                    }
                    button {
                        class: "btn-small",
                        disabled: busy,
                        title: "Send backward",
                        onclick: move |_| send_cmd(eval_handle, EditorCommand::Reorder { direction: LayerDirection::Backward }),
                        "Back"
                    }
                    button {
                        class: "btn-small btn-danger",
                        disabled: busy,
                        title: "Delete selected (Del)",
                        onclick: move |_| send_cmd(eval_handle, EditorCommand::DeleteSelected),
                        "Delete"
                    }
                }
            }

            // Context properties bar for the currently selected element
            if let Some(info) = sel {
                div { class: "editor-props", role: "toolbar", aria_label: "Element properties",
                    span { class: "prop-badge", "{info.label}" }
                    span { class: "muted prop-dim", "{info.width}×{info.height}" }

                    if info.is_text_editable {
                        div { class: "prop-group",
                            button {
                                class: if info.bold { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::Bold }),
                                strong { "B" }
                            }
                            button {
                                class: if info.italic { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::Italic }),
                                em { "I" }
                            }
                            button {
                                class: if info.underline { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::Underline }),
                                u { "U" }
                            }
                        }
                        div { class: "prop-group",
                            button {
                                class: if info.text_align == "left" { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                title: "Align left",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::AlignLeft }),
                                "L"
                            }
                            button {
                                class: if info.text_align == "center" { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                title: "Align center",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::AlignCenter }),
                                "C"
                            }
                            button {
                                class: if info.text_align == "right" { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                title: "Align right",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::ToggleFormat { cmd: FormatCommand::AlignRight }),
                                "R"
                            }
                        }
                        label { class: "prop-field", title: "Font size (px)",
                            span { class: "muted", "Size" }
                            input {
                                class: "prop-num",
                                r#type: "number",
                                min: "8",
                                max: "144",
                                disabled: busy,
                                value: "{info.font_size}",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse::<u32>() {
                                        if (8..=144).contains(&v) {
                                            send_cmd(eval_handle, EditorCommand::SetStyle {
                                                prop: "font_size".into(),
                                                value: v.to_string(),
                                            });
                                        }
                                    }
                                },
                            }
                        }
                        label { class: "prop-field", title: "Text color",
                            span { class: "muted", "Text" }
                            input {
                                class: "prop-color",
                                r#type: "color",
                                disabled: busy,
                                value: "{color_input_val(&info.color, \"#111827\")}",
                                oninput: move |e| send_cmd(eval_handle, EditorCommand::SetStyle {
                                    prop: "color".into(),
                                    value: e.value(),
                                }),
                            }
                        }
                    } else if info.kind == "arrow" {
                        label { class: "prop-field", title: "Arrow color",
                            span { class: "muted", "Color" }
                            input {
                                class: "prop-color",
                                r#type: "color",
                                disabled: busy,
                                value: "{color_input_val(&info.color, \"#1e293b\")}",
                                oninput: move |e| send_cmd(eval_handle, EditorCommand::SetStyle {
                                    prop: "color".into(),
                                    value: e.value(),
                                }),
                            }
                        }
                    }

                    label { class: "prop-field", title: "Fill / background color",
                        span { class: "muted", "Fill" }
                        input {
                            class: "prop-color",
                            r#type: "color",
                            disabled: busy,
                            value: "{color_input_val(&info.background, \"#ffffff\")}",
                            oninput: move |e| send_cmd(eval_handle, EditorCommand::SetStyle {
                                prop: "background".into(),
                                value: e.value(),
                            }),
                        }
                        button {
                            class: if info.background == "transparent" { "btn-small active" } else { "btn-small" },
                            disabled: busy,
                            title: "Transparent fill",
                            onclick: move |_| send_cmd(eval_handle, EditorCommand::SetStyle {
                                prop: "background".into(),
                                value: "transparent".into(),
                            }),
                            "∅"
                        }
                    }

                    label { class: "prop-field", title: "Border color & width",
                        span { class: "muted", "Border" }
                        input {
                            class: "prop-color",
                            r#type: "color",
                            disabled: busy,
                            value: "{color_input_val(&info.border_color, \"#16a34a\")}",
                            oninput: move |e| send_cmd(eval_handle, EditorCommand::SetStyle {
                                prop: "border_color".into(),
                                value: e.value(),
                            }),
                        }
                        input {
                            class: "prop-num",
                            r#type: "number",
                            min: "0",
                            max: "24",
                            disabled: busy,
                            value: "{info.border_width}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    send_cmd(eval_handle, EditorCommand::SetStyle {
                                        prop: "border_width".into(),
                                        value: v.to_string(),
                                    });
                                }
                            },
                        }
                    }

                    label { class: "prop-field", title: "Corner radius (px)",
                        span { class: "muted", "Rad" }
                        input {
                            class: "prop-num",
                            r#type: "number",
                            min: "0",
                            max: "200",
                            disabled: busy,
                            value: "{info.border_radius}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    send_cmd(eval_handle, EditorCommand::SetStyle {
                                        prop: "border_radius".into(),
                                        value: v.to_string(),
                                    });
                                }
                            },
                        }
                    }

                    label { class: "prop-field", title: "Opacity (%)",
                        span { class: "muted", "Op%" }
                        input {
                            class: "prop-num",
                            r#type: "number",
                            min: "10",
                            max: "100",
                            step: "5",
                            disabled: busy,
                            value: "{info.opacity}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    send_cmd(eval_handle, EditorCommand::SetStyle {
                                        prop: "opacity".into(),
                                        value: v.to_string(),
                                    });
                                }
                            },
                        }
                    }
                }
                if info.in_table {
                    div { class: "editor-props editor-table-bar", role: "toolbar", aria_label: "Table editor tools",
                        span { class: "prop-badge",
                            if info.cell_row > 0 && info.cell_col > 0 {
                                "Table {info.table_rows}×{info.table_cols} · R{info.cell_row} C{info.cell_col}"
                            } else {
                                "Table {info.table_rows}×{info.table_cols}"
                            }
                        }
                        div { class: "prop-group",
                            button {
                                class: "btn-small",
                                disabled: busy,
                                title: "Insert row above",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::InsertRowAbove }),
                                "+ Row ↑"
                            }
                            button {
                                class: "btn-small",
                                disabled: busy,
                                title: "Insert row below",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::InsertRowBelow }),
                                "+ Row ↓"
                            }
                            button {
                                class: "btn-small btn-danger",
                                disabled: busy || info.table_rows <= 1,
                                title: "Delete current row",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::DeleteRow }),
                                "- Row"
                            }
                        }
                        div { class: "prop-group",
                            button {
                                class: "btn-small",
                                disabled: busy,
                                title: "Insert column left",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::InsertColLeft }),
                                "+ Col ←"
                            }
                            button {
                                class: "btn-small",
                                disabled: busy,
                                title: "Insert column right",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::InsertColRight }),
                                "+ Col →"
                            }
                            button {
                                class: "btn-small btn-danger",
                                disabled: busy || info.table_cols <= 1,
                                title: "Delete current column",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::DeleteCol }),
                                "- Col"
                            }
                        }
                        if info.cell_row > 0 {
                            button {
                                class: if info.is_header_cell { "btn-small active" } else { "btn-small" },
                                disabled: busy,
                                title: "Toggle header cell (th / td)",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::ToggleHeader }),
                                "Header"
                            }
                            button {
                                class: "btn-small",
                                disabled: busy,
                                title: "Select whole table (move / resize / delete)",
                                onclick: move |_| send_cmd(eval_handle, EditorCommand::TableAction { action: TableActionKind::SelectTable }),
                                "Select Table"
                            }
                        }
                        span { class: "muted", style: "font-size: 11px;", "Tip: Double-click cell to edit, Tab for next cell" }
                    }
                }
            } else {
                div { class: "editor-hint",
                    "Click any element or table cell to edit · Double-click text to edit inline · Use the toolbar to insert text, shapes, images, or tables"
                }
            }

            div { class: "editor-canvas-wrap",
                iframe {
                    id: "visual-editor-frame",
                    class: "preview editor-frame",
                }
            }
        }
    }
}
