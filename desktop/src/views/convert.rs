//! Main workbench: Image-to-PDF or Prompt-to-PDF -> Visual Edit / HTML -> Export PDF.

use crate::{
    editor,
    ollama::{self, PullUpdate},
    pdf,
    settings::Settings,
    views::VisualEditor,
};
use base64::Engine;
use dioxus::prelude::*;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub enum Status {
    Idle,
    Busy(String),
    MissingModel(String),
    Downloading(PullUpdate),
    Ok(String),
    Error(String),
}

#[derive(Clone, PartialEq)]
pub struct LoadedImage {
    path: PathBuf,
    /// Original file bytes.
    bytes: Vec<u8>,
    /// `data:` URL used for the on-screen preview.
    preview: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    ImageToPdf,
    PromptToPdf,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Preview,
    Edit,
    Source,
}

/// Shared converter state kept at the app root so navigating away doesn't clear it.
#[derive(Clone, Copy)]
pub struct ConvertState {
    pub mode: Signal<Mode>,
    pub image: Signal<Option<LoadedImage>>,
    pub extra_prompt: Signal<String>,
    pub doc_prompt: Signal<String>,
    pub html: Signal<String>,
    pub status: Signal<Status>,
    pub tab: Signal<Tab>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            mode: Signal::new(Mode::ImageToPdf),
            image: Signal::new(None),
            extra_prompt: Signal::new(String::new()),
            doc_prompt: Signal::new(String::new()),
            html: Signal::new(String::new()),
            status: Signal::new(Status::Idle),
            tab: Signal::new(Tab::Preview),
        }
    }
}

async fn convert_loaded_image(
    mut cfg: Settings,
    extra_prompt: String,
    img: LoadedImage,
    mut html: Signal<String>,
    mut tab: Signal<Tab>,
    mut status: Signal<Status>,
) {
    let extra = extra_prompt.trim();
    if !extra.is_empty() {
        cfg.prompt = format!(
            "{}\n\nAdditional user instructions / customizations (apply these while still outputting only valid HTML):\n{}",
            cfg.prompt.trim_end(),
            extra
        );
    }

    status.set(Status::Busy(format!(
        "Running {} locally… this can take a few minutes.",
        cfg.model
    )));

    let max_dim = cfg.max_image_dim;
    let prepared =
        tokio::task::spawn_blocking(move || ollama::prepare_image(&img.bytes, max_dim)).await;
    let png = match prepared {
        Ok(Ok(png)) => png,
        Ok(Err(e)) => return status.set(Status::Error(format!("{e:#}"))),
        Err(e) => return status.set(Status::Error(e.to_string())),
    };

    match ollama::image_to_html(&cfg, &png).await {
        Ok(result) => {
            html.set(result);
            tab.set(Tab::Preview);
            status.set(Status::Ok("HTML generated. Review or edit it, then export to PDF.".into()));
        }
        Err(e) => status.set(Status::Error(format!("{e:#}"))),
    }
}

async fn generate_from_prompt(
    cfg: Settings,
    user_prompt: String,
    mut html: Signal<String>,
    mut tab: Signal<Tab>,
    mut status: Signal<Status>,
) {
    status.set(Status::Busy(format!("Generating document with {}…", cfg.model)));
    match ollama::prompt_to_html(&cfg, &user_prompt).await {
        Ok(result) => {
            html.set(result);
            tab.set(Tab::Preview);
            status.set(Status::Ok(
                "Document generated. Review or edit the HTML, then export to PDF.".into(),
            ));
        }
        Err(e) => status.set(Status::Error(format!("{e:#}"))),
    }
}

async fn ensure_model_ready(cfg: &Settings, mut status: Signal<Status>) -> bool {
    status.set(Status::Busy(format!("Checking if `{}` is installed in Ollama…", cfg.model)));
    match ollama::is_model_installed(&cfg.ollama_url, &cfg.model).await {
        Ok(true) => true,
        Ok(false) => {
            status.set(Status::MissingModel(cfg.model.clone()));
            false
        }
        Err(e) => {
            status.set(Status::Error(format!("{e:#}")));
            false
        }
    }
}

#[component]
pub fn Convert() -> Element {
    let settings = use_context::<Signal<Settings>>();
    let state = use_context::<ConvertState>();
    let mut mode = state.mode;
    let mut image = state.image;
    let mut extra_prompt = state.extra_prompt;
    let mut doc_prompt = state.doc_prompt;
    let mut html = state.html;
    let mut status = state.status;
    let mut tab = state.tab;
    let mut mode_menu_open = use_signal(|| false);

    let busy = matches!(status(), Status::Busy(_) | Status::Downloading(_));
    let has_image = image.read().is_some();
    let has_doc_prompt = !doc_prompt.read().trim().is_empty();
    let has_html = !html.read().trim().is_empty();

    // Reset transient Busy/Downloading status if the user navigates away mid-task.
    use_drop(move || {
        if matches!(*status.peek(), Status::Busy(_) | Status::Downloading(_)) {
            status.set(Status::Idle);
        }
    });

    // Proactively check whether the configured model is installed in Ollama.
    use_effect(move || {
        let cfg = settings();
        spawn(async move {
            match ollama::is_model_installed(&cfg.ollama_url, &cfg.model).await {
                Ok(false) => {
                    if matches!(status(), Status::Idle | Status::MissingModel(_)) {
                        status.set(Status::MissingModel(cfg.model));
                    }
                }
                Ok(true) => {
                    if matches!(status(), Status::MissingModel(_)) {
                        status.set(Status::Idle);
                    }
                }
                Err(_) => {}
            }
        });
    });

    // --- Actions -----------------------------------------------------------

    let pick_image = move |_| async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Choose a document image")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff"])
            .pick_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        let bytes = file.read().await;
        let preview = format!(
            "data:{};base64,{}",
            editor::mime_for(&path),
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        );
        image.set(Some(LoadedImage { path, bytes, preview }));
        html.set(String::new());
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let run_model = move |_| async move {
        let Some(img) = image() else { return };
        let cfg = settings();
        if !ensure_model_ready(&cfg, status).await {
            return;
        }
        convert_loaded_image(cfg, extra_prompt(), img, html, tab, status).await;
    };

    let run_prompt = move |_| async move {
        let prompt_text = doc_prompt();
        if prompt_text.trim().is_empty() {
            return;
        }
        let cfg = settings();
        if !ensure_model_ready(&cfg, status).await {
            return;
        }
        generate_from_prompt(cfg, prompt_text, html, tab, status).await;
    };

    let download_missing_model = move |_| async move {
        let cfg = settings();
        status.set(Status::Downloading(PullUpdate {
            text: format!("Starting download of `{}` from Ollama…", cfg.model),
            percent: Some(0),
        }));

        let pull_res = ollama::pull_model(&cfg.ollama_url, &cfg.model, |update| {
            status.set(Status::Downloading(update));
        })
        .await;

        match pull_res {
            Ok(()) => match mode() {
                Mode::ImageToPdf => {
                    if let Some(img) = image() {
                        convert_loaded_image(cfg, extra_prompt(), img, html, tab, status).await;
                    } else {
                        status.set(Status::Ok(format!(
                            "Model `{}` downloaded and ready. Open an image to convert.",
                            cfg.model
                        )));
                    }
                }
                Mode::PromptToPdf => {
                    if !doc_prompt.read().trim().is_empty() {
                        generate_from_prompt(cfg, doc_prompt(), html, tab, status).await;
                    } else {
                        status.set(Status::Ok(format!(
                            "Model `{}` downloaded and ready. Enter a prompt to generate.",
                            cfg.model
                        )));
                    }
                }
            },
            Err(e) => status.set(Status::Error(format!("{e:#}"))),
        }
    };

    let default_name = move |ext: &str| {
        if mode() == Mode::ImageToPdf {
            if let Some(stem) =
                image().and_then(|i| i.path.file_stem().map(|s| s.to_string_lossy().into_owned()))
            {
                return format!("{stem}.{ext}");
            }
        }
        format!("document.{ext}")
    };

    let export_pdf = move |_| async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Save PDF")
            .set_file_name(default_name("pdf"))
            .add_filter("PDF", &["pdf"])
            .save_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        status.set(Status::Busy("Rendering PDF with Chromium…".into()));
        let chromium = settings.read().chromium_path.clone();
        match pdf::html_to_pdf(&chromium, &html(), &path).await {
            Ok(()) => status.set(Status::Ok(format!("Saved {}", path.display()))),
            Err(e) => status.set(Status::Error(format!("{e:#}"))),
        }
    };

    let save_html = move |_| async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Save HTML")
            .set_file_name(default_name("html"))
            .add_filter("HTML", &["html", "htm"])
            .save_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        match tokio::fs::write(&path, html()).await {
            Ok(()) => status.set(Status::Ok(format!("Saved {}", path.display()))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let clear_image = move |_| {
        image.set(None);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let clear_prompt = move |_| {
        doc_prompt.set(String::new());
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let clear_output = move |_| {
        html.set(String::new());
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let clear_all = move |_| {
        match mode() {
            Mode::ImageToPdf => image.set(None),
            Mode::PromptToPdf => doc_prompt.set(String::new()),
        }
        html.set(String::new());
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let html_line_count = html.read().lines().count();

    // --- UI ----------------------------------------------------------------

    rsx! {
        div { class: "toolbar", role: "toolbar", aria_label: "Workbench actions",
            div { class: "toolbar-group",
                div { class: "mode-dropdown",
                    button {
                        class: "mode-select",
                        disabled: busy,
                        aria_haspopup: "listbox",
                        aria_expanded: "{mode_menu_open()}",
                        onclick: move |_| mode_menu_open.toggle(),
                        span {
                            if mode() == Mode::ImageToPdf {
                                "Image to PDF"
                            } else {
                                "Prompt to PDF"
                            }
                        }
                        span { class: "mode-caret", aria_hidden: "true", "▾" }
                    }
                    if mode_menu_open() && !busy {
                        div {
                            class: "mode-backdrop",
                            onclick: move |_| mode_menu_open.set(false),
                        }
                        div { class: "mode-menu", role: "listbox", aria_label: "Conversion mode",
                            button {
                                class: if mode() == Mode::ImageToPdf { "mode-option active" } else { "mode-option" },
                                role: "option",
                                aria_selected: "{mode() == Mode::ImageToPdf}",
                                onclick: move |_| {
                                    mode.set(Mode::ImageToPdf);
                                    mode_menu_open.set(false);
                                },
                                span { "Image to PDF" }
                                if mode() == Mode::ImageToPdf {
                                    span { class: "mode-check", "✓" }
                                }
                            }
                            button {
                                class: if mode() == Mode::PromptToPdf { "mode-option active" } else { "mode-option" },
                                role: "option",
                                aria_selected: "{mode() == Mode::PromptToPdf}",
                                onclick: move |_| {
                                    mode.set(Mode::PromptToPdf);
                                    mode_menu_open.set(false);
                                },
                                span { "Prompt to PDF" }
                                if mode() == Mode::PromptToPdf {
                                    span { class: "mode-check", "✓" }
                                }
                            }
                        }
                    }
                }

                span { class: "toolbar-divider", aria_hidden: "true" }

                if mode() == Mode::ImageToPdf {
                    button { disabled: busy, onclick: pick_image,
                        svg {
                            class: "icon",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.9",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
                            polyline { points: "17 8 12 3 7 8" }
                            line { x1: "12", y1: "3", x2: "12", y2: "15" }
                        }
                        "Open Image"
                    }
                    button {
                        class: "primary",
                        disabled: busy || !has_image,
                        onclick: run_model,
                        svg {
                            class: "icon",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.9",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            polygon { points: "13 2 3 14 12 14 11 22 21 10 12 10 13 2" }
                        }
                        "Convert to HTML"
                    }
                    button {
                        class: "ghost btn-danger",
                        disabled: busy || (!has_image && !has_html),
                        onclick: clear_all,
                        "Clear all"
                    }
                } else {
                    button {
                        class: "primary",
                        disabled: busy || !has_doc_prompt,
                        onclick: run_prompt,
                        svg {
                            class: "icon",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.9",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            polygon { points: "13 2 3 14 12 14 11 22 21 10 12 10 13 2" }
                        }
                        "Generate HTML"
                    }
                    button {
                        class: "ghost btn-danger",
                        disabled: busy || (!has_doc_prompt && !has_html),
                        onclick: clear_all,
                        "Clear all"
                    }
                }
            }

            div { class: "toolbar-group",
                button { disabled: busy || !has_html, onclick: save_html, "Save HTML" }
                button { class: "primary", disabled: busy || !has_html, onclick: export_pdf,
                    svg {
                        class: "icon",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "1.9",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                        polyline { points: "14 2 14 8 20 8" }
                        path { d: "M12 18v-6" }
                        path { d: "m9 15 3 3 3-3" }
                    }
                    "Export PDF"
                }
            }
        }

        if mode() == Mode::ImageToPdf {
            div { class: "custom-prompt-box",
                div { class: "custom-prompt-header",
                    label { r#for: "custom-instructions-input", "Custom instructions" }
                    span { class: "muted", "(appended to base prompt)" }
                    span { class: "spacer" }
                    if !extra_prompt.read().is_empty() {
                        button {
                            class: "btn-small ghost",
                            disabled: busy,
                            onclick: move |_| extra_prompt.set(String::new()),
                            "Clear"
                        }
                    }
                }
                textarea {
                    id: "custom-instructions-input",
                    class: "custom-prompt-input",
                    rows: "2",
                    disabled: busy,
                    placeholder: "Optional styling or extraction rules (e.g. \"Use a clean sans-serif font\", \"Translate headings to English\", \"Highlight totals\")…",
                    value: "{extra_prompt}",
                    oninput: move |e| extra_prompt.set(e.value()),
                }
            }
        }

        match status() {
            Status::Idle => rsx! {},
            Status::Busy(msg) => rsx! {
                div { class: "status busy", role: "status", aria_live: "polite",
                    span { class: "spinner" }
                    "{msg}"
                }
            },
            Status::MissingModel(model) => rsx! {
                div { class: "status warn", role: "status", aria_live: "polite",
                    span { "Model `{model}` is not installed in Ollama." }
                    span { class: "spacer" }
                    button {
                        class: "primary",
                        onclick: download_missing_model,
                        match mode() {
                            Mode::ImageToPdf if has_image => format!("Download `{model}` & Convert"),
                            Mode::PromptToPdf if has_doc_prompt => format!("Download `{model}` & Generate"),
                            _ => format!("Download `{model}`"),
                        }
                    }
                }
            },
            Status::Downloading(update) => rsx! {
                div { class: "status busy status-col", role: "status", aria_live: "polite",
                    div { class: "status-row",
                        span { class: "spinner" }
                        span { "{update.text}" }
                    }
                    if let Some(pct) = update.percent {
                        div { class: "progress-track",
                            div { class: "progress-fill", width: "{pct}%" }
                        }
                    }
                }
            },
            Status::Ok(msg) => rsx! {
                div { class: "status ok", role: "status", aria_live: "polite", "{msg}" }
            },
            Status::Error(msg) => rsx! {
                div { class: "status error", role: "alert", "{msg}" }
            },
        }

        div { class: "panes",
            // Left: source image or document prompt composer
            if mode() == Mode::ImageToPdf {
                section { class: "pane", aria_label: "Source document image",
                    header {
                        span { class: "pane-header-title", "Source image" }
                        if let Some(img) = image.read().as_ref() {
                            span { class: "muted path-label", "· {img.path.display()}" }
                        }
                        span { class: "spacer" }
                        if has_image {
                            button {
                                class: "btn-small ghost",
                                disabled: busy,
                                onclick: clear_image,
                                "Clear"
                            }
                        }
                    }
                    div { class: "pane-body image-body",
                        if let Some(img) = image.read().as_ref() {
                            img { src: "{img.preview}", alt: "Uploaded source document" }
                        } else {
                            button {
                                class: "empty-card",
                                disabled: busy,
                                onclick: pick_image,
                                div { class: "empty-icon", aria_hidden: "true",
                                    svg {
                                        width: "20",
                                        height: "20",
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.8",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        rect { x: "3", y: "3", width: "18", height: "18", rx: "2", ry: "2" }
                                        circle { cx: "8.5", cy: "8.5", r: "1.5" }
                                        polyline { points: "21 15 16 10 5 21" }
                                    }
                                }
                                span { class: "empty-title", "Select a document image" }
                                span { class: "empty-desc", "Choose a scanned page, invoice, receipt, or form to transcribe into HTML5." }
                                span { class: "empty-formats", "PNG · JPG · WEBP · TIFF · BMP" }
                            }
                        }
                    }
                }
            } else {
                section { class: "pane", aria_label: "Document generation prompt",
                    header {
                        span { class: "pane-header-title", "Document prompt" }
                        span { class: "spacer" }
                        if has_doc_prompt {
                            button {
                                class: "btn-small ghost",
                                disabled: busy,
                                onclick: clear_prompt,
                                "Clear"
                            }
                        }
                    }
                    div { class: "pane-body prompt-pane-body",
                        textarea {
                            class: "prompt-composer",
                            aria_label: "Document generation prompt",
                            disabled: busy,
                            placeholder: "Describe the document you want to generate...\n\nExample: Generate me a poultry farm bill of 10kg chicken with itemized rates, weight, subtotal, and total.",
                            value: "{doc_prompt}",
                            oninput: move |e| doc_prompt.set(e.value()),
                        }
                        div { class: "chips",
                            span { class: "muted", "Examples:" }
                            button {
                                class: "chip",
                                disabled: busy,
                                onclick: move |_| doc_prompt.set("Generate me a poultry farm bill of 10kg chicken with itemized rates, weight, subtotal, and total.".into()),
                                "Poultry farm bill (10 chickens)"
                            }
                            button {
                                class: "chip",
                                disabled: busy,
                                onclick: move |_| doc_prompt.set("Generate a freelance web development service invoice with 3 line items, tax, and payment terms".into()),
                                "Service invoice"
                            }
                            button {
                                class: "chip",
                                disabled: busy,
                                onclick: move |_| doc_prompt.set("Generate a monthly apartment rent receipt with landlord, tenant, period, amount, and signature line".into()),
                                "Rent receipt"
                            }
                        }
                    }
                }
            }

            // Right: generated HTML / visual editor
            section { class: "pane", aria_label: "Generated document output",
                header {
                    div { class: "toolbar-group", role: "tablist", aria_label: "Output views",
                        button {
                            class: if tab() == Tab::Preview { "tab active" } else { "tab" },
                            role: "tab",
                            aria_selected: "{tab() == Tab::Preview}",
                            onclick: move |_| tab.set(Tab::Preview),
                            "Preview"
                        }
                        if has_html {
                            button {
                                class: if tab() == Tab::Edit { "tab active" } else { "tab" },
                                role: "tab",
                                aria_selected: "{tab() == Tab::Edit}",
                                onclick: move |_| tab.set(Tab::Edit),
                                "Visual Edit"
                            }
                        }
                        button {
                            class: if tab() == Tab::Source { "tab active" } else { "tab" },
                            role: "tab",
                            aria_selected: "{tab() == Tab::Source}",
                            onclick: move |_| tab.set(Tab::Source),
                            "HTML source"
                        }
                    }
                    span { class: "spacer" }
                    if has_html {
                        span { class: "pane-badge", "{html_line_count} lines" }
                        button {
                            class: "btn-small ghost",
                            disabled: busy,
                            onclick: clear_output,
                            "Clear"
                        }
                    }
                }
                div { class: "pane-body",
                    if !has_html {
                        div { class: "empty-state",
                            div { class: "empty-icon", aria_hidden: "true",
                                svg {
                                    width: "20",
                                    height: "20",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.8",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                                    polyline { points: "14 2 14 8 20 8" }
                                    line { x1: "16", y1: "13", x2: "8", y2: "13" }
                                    line { x1: "16", y1: "17", x2: "8", y2: "17" }
                                }
                            }
                            span { class: "empty-title", "No HTML output yet" }
                            span { class: "empty-desc",
                                if mode() == Mode::ImageToPdf {
                                    "Open an image and click Convert to HTML. Once generated, Visual Edit and PDF export will unlock."
                                } else {
                                    "Enter a document prompt and click Generate HTML. Once generated, Visual Edit and PDF export will unlock."
                                }
                            }
                        }
                    } else if tab() == Tab::Preview {
                        iframe { class: "preview", srcdoc: "{html}", title: "HTML document preview" }
                    } else if tab() == Tab::Edit {
                        VisualEditor { html, busy, status }
                    } else {
                        textarea {
                            class: "editor",
                            aria_label: "HTML source code",
                            spellcheck: false,
                            value: "{html}",
                            oninput: move |e| html.set(e.value()),
                        }
                    }
                }
            }
        }
    }
}
