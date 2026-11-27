//! Main workbench: Image / Prompt / PDF -> HTML -> Visual Edit / Table / Refine AI -> Export PDF.

use crate::{
    clipboard,
    editor,
    image_ops,
    ollama::{self, ChatPhase, ChatProgress, PullUpdate},
    page_setup::{self, Orientation, PageSetup, PaperSize},
    pdf,
    pdf_import::{self, ExtractedPage, LoadedPdf},
    settings::{AiProvider, Settings},
    views::VisualEditor,
    Route,
};
use base64::Engine;
use dioxus::core::Task;
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub enum Status {
    Idle,
    Busy(String),
    MissingModel(String),
    MissingApiKey(AiProvider),
    Downloading(PullUpdate),
    Ok(String),
    Error(String),
}

#[derive(Clone, PartialEq)]
pub struct LoadedImage {
    pub path: PathBuf,
    /// Original unedited file bytes so Crop / Rotate / Deskew can be reset.
    pub original_bytes: Vec<u8>,
    /// Current working image bytes.
    pub bytes: Vec<u8>,
    /// `data:` URL used for the on-screen preview.
    pub preview: String,
    /// Whether any crop / rotate / deskew edit has been applied.
    pub modified: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    ImageToPdf,
    PromptToPdf,
    PdfToHtml,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Preview,
    Edit,
    Source,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Clone, PartialEq)]
pub struct ChatTurn {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkbenchJsEvent {
    PastedDataUrl { data_url: String },
    CheckOsClipboard,
    CropRect { x: f32, y: f32, w: f32, h: f32 },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkbenchJsCmd {
    Shutdown,
}

/// Shared converter state kept at the app root so navigating away doesn't clear it.
#[derive(Clone, Copy)]
pub struct ConvertState {
    pub mode: Signal<Mode>,
    pub image: Signal<Option<LoadedImage>>,
    pub pdf: Signal<Option<LoadedPdf>>,
    pub extra_prompt: Signal<String>,
    pub doc_prompt: Signal<String>,
    pub html: Signal<String>,
    pub status: Signal<Status>,
    pub tab: Signal<Tab>,
    pub page_setup: Signal<PageSetup>,
    pub chat_history: Signal<Vec<ChatTurn>>,
    pub refine_input: Signal<String>,
    pub refine_open: Signal<bool>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            mode: Signal::new(Mode::ImageToPdf),
            image: Signal::new(None),
            pdf: Signal::new(None),
            extra_prompt: Signal::new(String::new()),
            doc_prompt: Signal::new(String::new()),
            html: Signal::new(String::new()),
            status: Signal::new(Status::Idle),
            tab: Signal::new(Tab::Preview),
            page_setup: Signal::new(PageSetup::default()),
            chat_history: Signal::new(Vec::new()),
            refine_input: Signal::new(String::new()),
            refine_open: Signal::new(false),
        }
    }
}

const WORKBENCH_JS: &str = r##"
(function () {
  if (window.__dfWorkbenchCleanup) {
    try { window.__dfWorkbenchCleanup(); } catch (_) {}
  }

  let activeChannel = null;
  let cropDrag = null;
  let lastPasteAt = 0;
  let lastCropEmitAt = 0;

  function isTextTarget(el) {
    if (!el) return false;
    const tag = el.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || el.isContentEditable) return true;
    if (el.id === "visual-editor-frame") return true;
    return false;
  }

  function sendImageFile(file) {
    if (!file || !activeChannel) return false;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string" && activeChannel) {
        try {
          activeChannel.send({ type: "pasted_data_url", data_url: reader.result });
        } catch (_) {}
      }
    };
    reader.readAsDataURL(file);
    return true;
  }

  function onPaste(e) {
    if (!activeChannel) return;
    lastPasteAt = Date.now();
    const items = e.clipboardData && e.clipboardData.items;
    if (items) {
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        if (it.type && it.type.startsWith("image/")) {
          const file = it.getAsFile();
          if (file) {
            e.preventDefault();
            sendImageFile(file);
            return;
          }
        }
      }
    }
    const files = e.clipboardData && e.clipboardData.files;
    if (files) {
      for (let i = 0; i < files.length; i++) {
        const f = files[i];
        if (f && (!f.type || f.type.startsWith("image/"))) {
          e.preventDefault();
          sendImageFile(f);
          return;
        }
      }
    }
    if (!isTextTarget(document.activeElement)) {
      try {
        activeChannel.send({ type: "check_os_clipboard" });
      } catch (_) {}
    }
  }

  function onKeyDown(e) {
    if (!activeChannel) return;
    if ((e.ctrlKey || e.metaKey) && e.key && e.key.toLowerCase() === "v") {
      if (!isTextTarget(document.activeElement)) {
        setTimeout(() => {
          if (Date.now() - lastPasteAt > 120 && activeChannel) {
            try {
              activeChannel.send({ type: "check_os_clipboard" });
            } catch (_) {}
          }
        }, 80);
      }
    }
  }

  function clamp01(v) {
    return Math.max(0, Math.min(1, v));
  }

  function readCurrentCropRect(box) {
    if (!box) return { x: 0.08, y: 0.08, w: 0.84, h: 0.84 };
    const x = parseFloat(box.style.left);
    const y = parseFloat(box.style.top);
    const w = parseFloat(box.style.width);
    const h = parseFloat(box.style.height);
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(w) || !Number.isFinite(h)) {
      return { x: 0.08, y: 0.08, w: 0.84, h: 0.84 };
    }
    return {
      x: clamp01(x / 100),
      y: clamp01(y / 100),
      w: Math.max(0.05, clamp01(w / 100)),
      h: Math.max(0.05, clamp01(h / 100)),
    };
  }

  function computeUpdatedCropRect(drag, curPt) {
    const rect = drag.stage.getBoundingClientRect();
    if (rect.width < 4 || rect.height < 4) return null;
    const dx = (curPt.x - drag.start.x) / rect.width;
    const dy = (curPt.y - drag.start.y) / rect.height;
    const init = drag.initial;
    const minSize = 0.05;

    if (drag.mode === "move") {
      return {
        x: Math.max(0, Math.min(1 - init.w, init.x + dx)),
        y: Math.max(0, Math.min(1 - init.h, init.y + dy)),
        w: init.w,
        h: init.h,
      };
    }

    let left = init.x;
    let top = init.y;
    let right = init.x + init.w;
    let bottom = init.y + init.h;
    const handle = drag.handle || "";

    if (handle.includes("w")) {
      left = Math.max(0, Math.min(right - minSize, init.x + dx));
    }
    if (handle.includes("e")) {
      right = Math.min(1, Math.max(left + minSize, init.x + init.w + dx));
    }
    if (handle.includes("n")) {
      top = Math.max(0, Math.min(bottom - minSize, init.y + dy));
    }
    if (handle.includes("s")) {
      bottom = Math.min(1, Math.max(top + minSize, init.y + init.h + dy));
    }

    return {
      x: left,
      y: top,
      w: Math.max(minSize, right - left),
      h: Math.max(minSize, bottom - top),
    };
  }

  function previewCropDom(stage, cr) {
    const box = stage.querySelector(".crop-box");
    if (!box || !cr) return;
    box.style.left = `${cr.x * 100}%`;
    box.style.top = `${cr.y * 100}%`;
    box.style.width = `${cr.w * 100}%`;
    box.style.height = `${cr.h * 100}%`;
    const badge = box.querySelector(".crop-badge");
    if (badge) {
      badge.textContent = `${Math.round(cr.w * 100)}% × ${Math.round(cr.h * 100)}%`;
    }
  }

  function emitCropUpdate(drag, curPt, forceIpc) {
    const cr = computeUpdatedCropRect(drag, curPt);
    if (!cr) return;
    previewCropDom(drag.stage, cr);
    const now = Date.now();
    if (activeChannel && (forceIpc || now - lastCropEmitAt >= 80)) {
      lastCropEmitAt = now;
      try {
        activeChannel.send({ type: "crop_rect", ...cr });
      } catch (_) {}
    }
  }

  function onMouseDown(e) {
    if (e.button !== 0 || !e.target || !e.target.closest) return;
    const stage = e.target.closest("#df-crop-stage");
    if (!stage) return;
    const handleEl = e.target.closest("[data-crop-h]");
    const box = e.target.closest(".crop-box");
    if (!handleEl && !box) return;
    e.preventDefault();
    const targetBox = box || stage.querySelector(".crop-box");
    cropDrag = {
      stage,
      mode: handleEl ? "resize" : "move",
      handle: handleEl ? handleEl.getAttribute("data-crop-h") : "",
      start: { x: e.clientX, y: e.clientY },
      initial: readCurrentCropRect(targetBox),
    };
  }

  function onMouseMove(e) {
    if (!cropDrag) return;
    emitCropUpdate(cropDrag, { x: e.clientX, y: e.clientY }, false);
  }

  function onMouseUp(e) {
    if (!cropDrag) return;
    const dx = Math.abs(e.clientX - cropDrag.start.x);
    const dy = Math.abs(e.clientY - cropDrag.start.y);
    if (dx >= 2 || dy >= 2) {
      emitCropUpdate(cropDrag, { x: e.clientX, y: e.clientY }, true);
    }
    cropDrag = null;
  }

  window.addEventListener("paste", onPaste, true);
  window.addEventListener("keydown", onKeyDown, true);
  window.addEventListener("mousedown", onMouseDown, true);
  window.addEventListener("mousemove", onMouseMove, true);
  window.addEventListener("mouseup", onMouseUp, true);

  window.__dfWorkbenchCleanup = function () {
    window.removeEventListener("paste", onPaste, true);
    window.removeEventListener("keydown", onKeyDown, true);
    window.removeEventListener("mousedown", onMouseDown, true);
    window.removeEventListener("mousemove", onMouseMove, true);
    window.removeEventListener("mouseup", onMouseUp, true);
    activeChannel = null;
  };

  window.__dfStartWorkbench = async function (dioxusChannel) {
    activeChannel = dioxusChannel;
    while (true) {
      const cmd = await dioxusChannel.recv();
      if (!cmd || cmd.type === "shutdown") break;
    }
    if (window.__dfWorkbenchCleanup) {
      window.__dfWorkbenchCleanup();
    }
  };
})();
"##;

struct TempFileDrop(Option<PathBuf>);

impl Drop for TempFileDrop {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = std::fs::remove_file(p);
        }
    }
}

fn start_cancellable_task<F>(
    mut active_task: Signal<Option<(u64, Task)>>,
    mut task_seq: Signal<u64>,
    fut: F,
) where
    F: std::future::Future<Output = ()> + 'static,
{
    if let Some((_, prev)) = active_task.write().take() {
        prev.cancel();
    }
    let my_id = *task_seq.peek() + 1;
    task_seq.set(my_id);
    let finished = std::rc::Rc::new(std::cell::Cell::new(false));
    let finished_inner = finished.clone();
    let task = spawn(async move {
        fut.await;
        finished_inner.set(true);
        if active_task.peek().is_some_and(|(id, _)| id == my_id) {
            active_task.set(None);
        }
    });
    if !finished.get() && *task_seq.peek() == my_id {
        active_task.set(Some((my_id, task)));
    }
}

fn format_ai_progress(prefix: &str, model: &str, p: &ChatProgress) -> String {
    match p.phase {
        ChatPhase::Waiting => format!("{prefix}Waiting for {model}… ({}s)", p.elapsed_secs),
        ChatPhase::Thinking => format!(
            "{prefix}{model} is thinking… ({} chars · {}s)",
            p.thinking_chars, p.elapsed_secs
        ),
        ChatPhase::Generating => format!(
            "{prefix}Generating HTML with {model}… ({} chars · {}s)",
            p.chars, p.elapsed_secs
        ),
    }
}

async fn convert_image_bytes(
    mut cfg: Settings,
    extra_prompt: String,
    bytes: Vec<u8>,
    mut state: ConvertState,
) {
    let extra = extra_prompt.trim();
    if !extra.is_empty() {
        cfg.prompt = format!(
            "{}\n\nAdditional user instructions / customizations (apply these while still outputting only valid HTML):\n{}",
            cfg.prompt.trim_end(),
            extra
        );
    }

    state.status.set(Status::Busy(format!(
        "Preparing image and contacting {}…",
        cfg.active_model()
    )));

    let max_dim = cfg.max_image_dim;
    let prepared =
        tokio::task::spawn_blocking(move || ollama::prepare_image(&bytes, max_dim)).await;
    let prepared_bytes = match prepared {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => return state.status.set(Status::Error(format!("{e:#}"))),
        Err(e) => return state.status.set(Status::Error(e.to_string())),
    };

    let model_name = cfg.active_model().to_string();
    let mut status_sig = state.status;
    match ollama::image_to_html_streaming(&cfg, &prepared_bytes, move |p| {
        status_sig.set(Status::Busy(format_ai_progress("", &model_name, &p)));
    })
    .await
    {
        Ok(result) => {
            let setup = (state.page_setup)();
            state.html.set(page_setup::apply_page_setup(&result, &setup));
            state.chat_history.set(Vec::new());
            state.tab.set(Tab::Preview);
            state.status.set(Status::Ok(
                "HTML generated. Review, edit, or refine with AI, then export to PDF.".into(),
            ));
        }
        Err(e) => state.status.set(Status::Error(format!("{e:#}"))),
    }
}

async fn generate_from_prompt(cfg: Settings, user_prompt: String, mut state: ConvertState) {
    let model_name = cfg.active_model().to_string();
    state
        .status
        .set(Status::Busy(format!("Generating document with {model_name}…")));
    let mut status_sig = state.status;
    match ollama::prompt_to_html_streaming(&cfg, &user_prompt, move |p| {
        status_sig.set(Status::Busy(format_ai_progress("", &model_name, &p)));
    })
    .await
    {
        Ok(result) => {
            let setup = (state.page_setup)();
            state.html.set(page_setup::apply_page_setup(&result, &setup));
            state.chat_history.set(Vec::new());
            state.tab.set(Tab::Preview);
            state.status.set(Status::Ok(
                "Document generated. Review, edit, or refine with AI, then export to PDF.".into(),
            ));
        }
        Err(e) => state.status.set(Status::Error(format!("{e:#}"))),
    }
}

async fn convert_pdf_with_ai(
    mut cfg: Settings,
    extra_prompt: String,
    pdf: &LoadedPdf,
    plan: &pdf_import::PdfExtractionPlan,
    mut state: ConvertState,
) {
    let extra = extra_prompt.trim();
    if !extra.is_empty() {
        cfg.prompt = format!(
            "{}\n\nAdditional user instructions / customizations (apply these while still outputting only valid HTML):\n{}",
            cfg.prompt.trim_end(),
            extra
        );
    }

    let ocr_indices = plan.image_only_pages();
    let total_ocr = ocr_indices.len().max(1);
    let mut ocr_done = 0usize;
    let mut rendered_pages = Vec::with_capacity(plan.pages.len());

    for page_plan in &plan.pages {
        match page_plan {
            ExtractedPage::Structured(sec) => rendered_pages.push(sec.clone()),
            ExtractedPage::NeedsOcr { page_index } => {
                let Some(pdf_page) = pdf.pages.get(*page_index) else {
                    continue;
                };
                ocr_done += 1;
                let prefix = format!("Page {ocr_done}/{total_ocr}: ");
                let model_name = cfg.active_model().to_string();
                state.status.set(Status::Busy(format!(
                    "{prefix}Preparing image and contacting {model_name}…"
                )));
                let raw_bytes = pdf_page.png_bytes.clone();
                let max_dim = cfg.max_image_dim;
                let prepared =
                    tokio::task::spawn_blocking(move || ollama::prepare_image(&raw_bytes, max_dim))
                        .await;
                let prepared_bytes = match prepared {
                    Ok(Ok(bytes)) => bytes,
                    Ok(Err(e)) => return state.status.set(Status::Error(format!("{e:#}"))),
                    Err(e) => return state.status.set(Status::Error(e.to_string())),
                };
                let mut status_sig = state.status;
                match ollama::image_to_html_streaming(&cfg, &prepared_bytes, move |p| {
                    status_sig.set(Status::Busy(format_ai_progress(&prefix, &model_name, &p)));
                })
                .await
                {
                    Ok(page_html) => rendered_pages.push(page_html),
                    Err(e) => return state.status.set(Status::Error(format!("{e:#}"))),
                }
            }
        }
    }

    let combined = pdf_import::combine_html_pages(&rendered_pages);
    if combined.trim().is_empty() {
        return state.status.set(Status::Error(
            "Could not extract any editable content from this PDF.".into(),
        ));
    }

    let setup = (state.page_setup)();
    state
        .html
        .set(page_setup::apply_page_setup(&combined, &setup));
    state.chat_history.set(Vec::new());
    state.tab.set(Tab::Preview);
    state.status.set(Status::Ok(
        "Converted scanned PDF to editable HTML. Switch to Visual Edit to edit any element.".into(),
    ));
}

async fn ensure_model_ready(cfg: &Settings, mut status: Signal<Status>) -> bool {
    if cfg.provider.is_cloud() {
        if cfg.active_model().trim().is_empty() {
            status.set(Status::Error("Model name cannot be empty. Configure it in Settings.".into()));
            return false;
        }
        if cfg.provider.requires_api_key() && cfg.effective_api_key().0.is_empty() {
            status.set(Status::MissingApiKey(cfg.provider));
            return false;
        }
        if cfg.provider == AiProvider::OpenAiCompatible && cfg.custom_api_url.trim().is_empty() {
            status.set(Status::Error("Enter an API Base URL in Settings.".into()));
            return false;
        }
        return true;
    }

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

fn make_loaded_image(path: PathBuf, bytes: Vec<u8>) -> LoadedImage {
    let preview = format!(
        "data:{};base64,{}",
        editor::mime_for(&path),
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    LoadedImage {
        path,
        original_bytes: bytes.clone(),
        bytes,
        preview,
        modified: false,
    }
}

fn update_loaded_image_bytes(img: &LoadedImage, new_png: Vec<u8>, modified: bool) -> LoadedImage {
    let preview = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&new_png)
    );
    LoadedImage {
        path: img.path.clone(),
        original_bytes: img.original_bytes.clone(),
        bytes: new_png,
        preview,
        modified,
    }
}

async fn run_pick_image(
    mut state: ConvertState,
    mut crop_mode: Signal<bool>,
    mut straighten_deg: Signal<f32>,
) {
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
    let loaded = make_loaded_image(path, bytes);
    state.mode.set(Mode::ImageToPdf);
    crop_mode.set(false);
    straighten_deg.set(0.0);
    state.image.set(Some(loaded));
    state.html.set(String::new());
    state.chat_history.set(Vec::new());
    state.refine_open.set(false);
    state.tab.set(Tab::Preview);
    if !matches!(*state.status.peek(), Status::MissingModel(_)) {
        state.status.set(Status::Idle);
    }
}

async fn run_paste_clipboard(
    mut state: ConvertState,
    mut crop_mode: Signal<bool>,
    mut straighten_deg: Signal<f32>,
) {
    state.status.set(Status::Busy("Reading image from clipboard…".into()));
    match clipboard::read_os_clipboard_image().await {
        Ok(raw) => match clipboard::normalize_clipboard_image(&raw) {
            Ok((png, preview)) => {
                state.mode.set(Mode::ImageToPdf);
                crop_mode.set(false);
                straighten_deg.set(0.0);
                state.image.set(Some(LoadedImage {
                    path: PathBuf::from("clipboard.png"),
                    original_bytes: png.clone(),
                    bytes: png,
                    preview,
                    modified: false,
                }));
                state.html.set(String::new());
                state.chat_history.set(Vec::new());
                state.refine_open.set(false);
                state.tab.set(Tab::Preview);
                state.status.set(Status::Ok("Loaded image from clipboard.".into()));
            }
            Err(e) => state.status.set(Status::Error(format!("{e:#}"))),
        },
        Err(e) => state.status.set(Status::Error(format!("{e:#}"))),
    }
}

#[component]
pub fn Convert() -> Element {
    let settings = use_context::<Signal<Settings>>();
    let state = use_context::<ConvertState>();
    let mut mode = state.mode;
    let mut image = state.image;
    let mut pdf_doc = state.pdf;
    let mut extra_prompt = state.extra_prompt;
    let mut doc_prompt = state.doc_prompt;
    let mut html = state.html;
    let mut status = state.status;
    let mut tab = state.tab;
    let page_setup = state.page_setup;
    let mut chat_history = state.chat_history;
    let mut refine_input = state.refine_input;
    let mut refine_open = state.refine_open;

    let mut mode_menu_open = use_signal(|| false);
    let mut page_menu_open = use_signal(|| false);
    let mut drag_active = use_signal(|| false);
    let mut crop_mode = use_signal(|| false);
    let mut crop_rect = use_signal(|| (0.08f32, 0.08f32, 0.84f32, 0.84f32));
    let mut straighten_deg = use_signal(|| 0.0f32);
    let mut workbench_eval = use_signal(|| None::<document::Eval>);
    let mut active_task = use_signal(|| None::<(u64, Task)>);
    let mut task_seq = use_signal(|| 0u64);

    let busy = matches!(status(), Status::Busy(_) | Status::Downloading(_));
    let has_image = image.read().is_some();
    let has_pdf = pdf_doc.read().is_some();
    let is_scanned_pdf = pdf_doc.read().as_ref().is_some_and(|p| !p.has_selectable_text());
    let has_doc_prompt = !doc_prompt.read().trim().is_empty();
    let has_html = !html.read().trim().is_empty();

    // Reset transient Busy/Downloading status and cancel active tasks + JS listeners on unmount.
    use_drop(move || {
        let next = *task_seq.peek() + 1;
        task_seq.set(next);
        if let Some((_, task)) = active_task.write().take() {
            task.cancel();
        }
        if matches!(*status.peek(), Status::Busy(_) | Status::Downloading(_)) {
            status.set(Status::Idle);
        }
        if let Some(ev) = *workbench_eval.peek() {
            let _ = ev.send(WorkbenchJsCmd::Shutdown);
        }
    });

    let cancel_generation = move |_| {
        let next = *task_seq.peek() + 1;
        task_seq.set(next);
        if let Some((_, task)) = active_task.write().take() {
            task.cancel();
        }
        status.set(Status::Ok("Generation cancelled.".into()));
    };

    // Mount global clipboard paste & drag-crop pointer controller once.
    use_effect(move || {
        if workbench_eval.peek().is_some() {
            return;
        }
        let script = format!("{WORKBENCH_JS}\nawait window.__dfStartWorkbench(dioxus);");
        let mut eval = document::eval(&script);
        workbench_eval.set(Some(eval));

        spawn(async move {
            while let Ok(evt) = eval.recv::<WorkbenchJsEvent>().await {
                match evt {
                    WorkbenchJsEvent::PastedDataUrl { data_url } => {
                        match clipboard::decode_base64_image(&data_url) {
                            Ok((png, preview)) => {
                                mode.set(Mode::ImageToPdf);
                                crop_mode.set(false);
                                crop_rect.set((0.08, 0.08, 0.84, 0.84));
                                straighten_deg.set(0.0);
                                image.set(Some(LoadedImage {
                                    path: PathBuf::from("clipboard.png"),
                                    original_bytes: png.clone(),
                                    bytes: png,
                                    preview,
                                    modified: false,
                                }));
                                html.set(String::new());
                                chat_history.set(Vec::new());
                                refine_open.set(false);
                                tab.set(Tab::Preview);
                                status.set(Status::Ok("Pasted image from clipboard.".into()));
                            }
                            Err(e) => status.set(Status::Error(format!("{e:#}"))),
                        }
                    }
                    WorkbenchJsEvent::CheckOsClipboard => {
                        if let Ok(raw) = clipboard::read_os_clipboard_image().await {
                            match clipboard::normalize_clipboard_image(&raw) {
                                Ok((png, preview)) => {
                                    mode.set(Mode::ImageToPdf);
                                    crop_mode.set(false);
                                    crop_rect.set((0.08, 0.08, 0.84, 0.84));
                                    straighten_deg.set(0.0);
                                    image.set(Some(LoadedImage {
                                        path: PathBuf::from("clipboard.png"),
                                        original_bytes: png.clone(),
                                        bytes: png,
                                        preview,
                                        modified: false,
                                    }));
                                    html.set(String::new());
                                    chat_history.set(Vec::new());
                                    refine_open.set(false);
                                    tab.set(Tab::Preview);
                                    status.set(Status::Ok("Pasted image from clipboard.".into()));
                                }
                                Err(e) => status.set(Status::Error(format!("{e:#}"))),
                            }
                        }
                    }
                    WorkbenchJsEvent::CropRect { x, y, w, h } => {
                        crop_rect.set((x, y, w, h));
                    }
                }
            }
        });
    });

    // Proactively validate active provider configuration / local Ollama model.
    use_effect(move || {
        let cfg = settings();
        if cfg.provider.is_cloud() {
            if cfg.provider.requires_api_key() && cfg.effective_api_key().0.is_empty() {
                if matches!(
                    status(),
                    Status::Idle | Status::MissingModel(_) | Status::MissingApiKey(_)
                ) {
                    status.set(Status::MissingApiKey(cfg.provider));
                }
            } else if matches!(status(), Status::MissingModel(_) | Status::MissingApiKey(_)) {
                status.set(Status::Idle);
            }
            return;
        }
        if matches!(status(), Status::MissingApiKey(_)) {
            status.set(Status::Idle);
        }
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

    // --- Shared Loaders ----------------------------------------------------

    let load_image_from_bytes = move |path: PathBuf, bytes: Vec<u8>| {
        let mut mode = mode;
        let mut crop_mode = crop_mode;
        let mut straighten_deg = straighten_deg;
        let mut image = image;
        let mut html = html;
        let mut chat_history = chat_history;
        let mut refine_open = refine_open;
        let mut tab = tab;
        let mut status = status;
        let loaded = make_loaded_image(path, bytes);
        mode.set(Mode::ImageToPdf);
        crop_mode.set(false);
        straighten_deg.set(0.0);
        image.set(Some(loaded));
        html.set(String::new());
        chat_history.set(Vec::new());
        refine_open.set(false);
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let load_pdf_from_path = move |path: PathBuf, display_name: Option<PathBuf>, remove_after: bool| {
        let active_task = active_task;
        let task_seq = task_seq;
        let mut status = status;
        let mut mode = mode;
        let mut pdf_doc = pdf_doc;
        let mut html = html;
        let mut chat_history = chat_history;
        let mut refine_open = refine_open;
        let mut tab = tab;
        status.set(Status::Busy("Importing PDF pages and extracting layout…".into()));
        start_cancellable_task(active_task, task_seq, async move {
            let _tmp_guard = TempFileDrop(remove_after.then(|| path.clone()));
            let res = pdf_import::import_pdf(&path).await;
            match res {
                Ok(mut loaded) => {
                    if let Some(disp) = display_name {
                        loaded.path = disp;
                    }
                    let n = loaded.page_count();
                    let selectable = loaded.has_selectable_text();
                    let scanned_count = loaded.image_only_page_count();
                    mode.set(Mode::PdfToHtml);
                    pdf_doc.set(Some(loaded));
                    html.set(String::new());
                    chat_history.set(Vec::new());
                    refine_open.set(false);
                    tab.set(Tab::Preview);
                    if !selectable {
                        let action_hint = if pdf_import::has_tesseract() {
                            "Click Convert Page (AI) for full layout & styling, or Fast OCR (Text) for instant offline text extraction."
                        } else {
                            "Click Convert Page (AI) to reconstruct layout & editable HTML."
                        };
                        status.set(Status::Ok(format!(
                            "Loaded scanned PDF ({n} page{}, image-only). {action_hint}",
                            if n == 1 { "" } else { "s" }
                        )));
                    } else if scanned_count > 0 {
                        status.set(Status::Ok(format!(
                            "Loaded hybrid PDF ({n} pages, {scanned_count} scanned). Click Quick Extract HTML to convert all pages."
                        )));
                    } else {
                        status.set(Status::Ok(format!(
                            "Loaded PDF ({n} page{}). Click Quick Extract HTML or Convert Page (AI).",
                            if n == 1 { "" } else { "s" }
                        )));
                    }
                }
                Err(e) => status.set(Status::Error(format!("{e:#}"))),
            }
        });
    };

    // --- Actions -----------------------------------------------------------

    let pick_image = move |_| run_pick_image(state, crop_mode, straighten_deg);

    let pick_pdf = move |_| async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Import a PDF document")
            .add_filter("PDF Documents", &["pdf"])
            .pick_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        load_pdf_from_path(path, None, false);
    };

    let paste_from_clipboard = move |_| run_paste_clipboard(state, crop_mode, straighten_deg);

    let handle_drop = move |evt: DragEvent| async move {
        evt.prevent_default();
        drag_active.set(false);
        if busy {
            return;
        }
        let files = evt.files();
        let Some(file) = files.into_iter().next() else {
            return;
        };
        let path = file.path();
        let name = file.name().to_ascii_lowercase();
        let bytes = match file.read_bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                status.set(Status::Error(format!("Failed to read dropped file: {e}")));
                return;
            }
        };

        if name.ends_with(".pdf") || bytes.starts_with(b"%PDF-") {
            if path.is_file() {
                load_pdf_from_path(path, None, false);
            } else {
                // Write dropped PDF bytes to a temp path for Poppler tools
                let tmp = std::env::temp_dir().join(format!(
                    "df_drop_{}.pdf",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                if let Err(e) = tokio::fs::write(&tmp, &bytes).await {
                    status.set(Status::Error(format!("Failed to stage dropped PDF: {e}")));
                    return;
                }
                let disp = if path.as_os_str().is_empty() {
                    Some(PathBuf::from(file.name()))
                } else {
                    Some(path)
                };
                load_pdf_from_path(tmp, disp, true);
            }
            return;
        }

        match image::load_from_memory(&bytes) {
            Ok(_) => {
                let display_path = if path.as_os_str().is_empty() {
                    PathBuf::from(file.name())
                } else {
                    path
                };
                load_image_from_bytes(display_path, bytes);
                status.set(Status::Ok("Loaded dropped image.".into()));
            }
            Err(_) => {
                status.set(Status::Error(
                    "Unsupported dropped file. Drop a PNG, JPG, WEBP, BMP, GIF, TIFF, or PDF.".into(),
                ));
            }
        }
    };

    // --- Image Crop / Rotate / Deskew Actions ------------------------------

    let rotate_cw = move |_| async move {
        let Some(img) = image() else { return };
        status.set(Status::Busy("Rotating image 90° clockwise…".into()));
        let bytes = img.bytes.clone();
        match tokio::task::spawn_blocking(move || image_ops::rotate_90_cw(&bytes)).await {
            Ok(Ok(out)) => {
                image.set(Some(update_loaded_image_bytes(&img, out, true)));
                crop_mode.set(false);
                crop_rect.set((0.08, 0.08, 0.84, 0.84));
                status.set(Status::Ok("Rotated image 90° clockwise.".into()));
            }
            Ok(Err(e)) => status.set(Status::Error(format!("{e:#}"))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let rotate_ccw = move |_| async move {
        let Some(img) = image() else { return };
        status.set(Status::Busy("Rotating image 90° counter-clockwise…".into()));
        let bytes = img.bytes.clone();
        match tokio::task::spawn_blocking(move || image_ops::rotate_90_ccw(&bytes)).await {
            Ok(Ok(out)) => {
                image.set(Some(update_loaded_image_bytes(&img, out, true)));
                crop_mode.set(false);
                crop_rect.set((0.08, 0.08, 0.84, 0.84));
                status.set(Status::Ok("Rotated image 90° counter-clockwise.".into()));
            }
            Ok(Err(e)) => status.set(Status::Error(format!("{e:#}"))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let auto_deskew = move |_| async move {
        let Some(img) = image() else { return };
        status.set(Status::Busy("Detecting skew angle and straightening image…".into()));
        let bytes = img.bytes.clone();
        match tokio::task::spawn_blocking(move || image_ops::deskew(&bytes)).await {
            Ok(Ok((out, angle))) => {
                if angle.abs() < 0.25 {
                    status.set(Status::Ok("Image is already straight (no skew detected).".into()));
                } else {
                    image.set(Some(update_loaded_image_bytes(&img, out, true)));
                    crop_mode.set(false);
                    crop_rect.set((0.08, 0.08, 0.84, 0.84));
                    status.set(Status::Ok(format!("Auto-deskewed image by {angle:+.1}°.")));
                }
            }
            Ok(Err(e)) => status.set(Status::Error(format!("{e:#}"))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let apply_straighten = move |_| async move {
        let Some(img) = image() else { return };
        let deg = straighten_deg();
        if deg.abs() < 0.1 {
            return;
        }
        status.set(Status::Busy(format!("Rotating image by {deg:+.1}°…")));
        let bytes = img.bytes.clone();
        match tokio::task::spawn_blocking(move || image_ops::rotate_angle(&bytes, deg)).await {
            Ok(Ok(out)) => {
                image.set(Some(update_loaded_image_bytes(&img, out, true)));
                straighten_deg.set(0.0);
                crop_mode.set(false);
                crop_rect.set((0.08, 0.08, 0.84, 0.84));
                status.set(Status::Ok(format!("Rotated image by {deg:+.1}°.")));
            }
            Ok(Err(e)) => status.set(Status::Error(format!("{e:#}"))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let apply_crop = move |_| async move {
        let Some(img) = image() else { return };
        status.set(Status::Busy("Applying crop to source image…".into()));
        let (x, y, w, h) = crop_rect();
        let bytes = img.bytes.clone();
        match tokio::task::spawn_blocking(move || image_ops::crop_normalized(&bytes, x, y, w, h)).await {
            Ok(Ok(out)) => {
                image.set(Some(update_loaded_image_bytes(&img, out, true)));
                crop_mode.set(false);
                crop_rect.set((0.08, 0.08, 0.84, 0.84));
                status.set(Status::Ok("Applied crop to source image.".into()));
            }
            Ok(Err(e)) => status.set(Status::Error(format!("{e:#}"))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let reset_image_edits = move |_| {
        if let Some(img) = image() {
            let restored = make_loaded_image(img.path, img.original_bytes);
            image.set(Some(restored));
            crop_mode.set(false);
            crop_rect.set((0.08, 0.08, 0.84, 0.84));
            straighten_deg.set(0.0);
            status.set(Status::Ok("Restored original source image.".into()));
        }
    };

    // --- Conversion & AI Actions -------------------------------------------

    let run_model = move |_| {
        let Some(img) = image() else { return };
        let cfg = settings();
        let extra = extra_prompt();
        let init_msg = if cfg.provider.is_cloud() {
            format!("Preparing image for {} ({})…", cfg.active_model(), cfg.provider.short_label())
        } else {
            format!("Checking if `{}` is installed in Ollama…", cfg.model)
        };
        status.set(Status::Busy(init_msg));
        start_cancellable_task(active_task, task_seq, async move {
            if ensure_model_ready(&cfg, status).await {
                convert_image_bytes(cfg, extra, img.bytes, state).await;
            }
        });
    };

    let run_prompt = move |_| {
        let prompt_text = doc_prompt();
        if prompt_text.trim().is_empty() {
            return;
        }
        let cfg = settings();
        let init_msg = if cfg.provider.is_cloud() {
            format!("Contacting {} ({})…", cfg.active_model(), cfg.provider.short_label())
        } else {
            format!("Checking if `{}` is installed in Ollama…", cfg.model)
        };
        status.set(Status::Busy(init_msg));
        start_cancellable_task(active_task, task_seq, async move {
            if ensure_model_ready(&cfg, status).await {
                generate_from_prompt(cfg, prompt_text, state).await;
            }
        });
    };

    let quick_extract_pdf = move |_| {
        let Some(pdf) = pdf_doc() else { return };
        let plan = pdf_import::plan_pdf_extraction(&pdf);
        let ocr_indices = plan.image_only_pages();

        if ocr_indices.is_empty() && plan.has_any_text() {
            if let Some(extracted) = plan.render_structured_only() {
                let styled = page_setup::apply_page_setup(&extracted, &page_setup());
                html.set(styled);
                chat_history.set(Vec::new());
                tab.set(Tab::Preview);
                status.set(Status::Ok(
                    "Extracted editable HTML from PDF. Switch to Visual Edit or Refine with AI."
                        .into(),
                ));
                return;
            }
        }

        let cfg = settings();
        let extra = extra_prompt();
        status.set(Status::Busy("Preparing PDF conversion…".into()));

        start_cancellable_task(active_task, task_seq, async move {
            // Prioritize fast parallel local Tesseract OCR (sub-second per page) before slower Vision AI.
            if pdf_import::has_tesseract() {
                let ocr_inputs: Vec<(usize, Vec<u8>)> = ocr_indices
                    .iter()
                    .filter_map(|&idx| pdf.pages.get(idx).map(|p| (idx, p.png_bytes.clone())))
                    .collect();

                if ocr_inputs.len() == ocr_indices.len() && !ocr_inputs.is_empty() {
                    let n_ocr = ocr_inputs.len();
                    status.set(Status::Busy(format!(
                        "Running fast offline OCR on {n_ocr} scanned page{}…",
                        if n_ocr == 1 { "" } else { "s" }
                    )));
                    let mut progress_status = status;
                    if let Ok(ocr_results) = pdf_import::ocr_pages_parallel_with_progress(
                        &ocr_inputs,
                        move |done, total| {
                            progress_status.set(Status::Busy(format!(
                                "Running fast offline OCR… {done}/{total} scanned pages"
                            )));
                        },
                    )
                    .await
                    {
                        let mut rendered_pages = Vec::with_capacity(plan.pages.len());
                        let mut extracted_any_text = false;
                        for page_plan in &plan.pages {
                            match page_plan {
                                ExtractedPage::Structured(sec) => {
                                    extracted_any_text = true;
                                    rendered_pages.push(sec.clone());
                                }
                                ExtractedPage::NeedsOcr { page_index } => {
                                    if let Some((_, Some(sec))) =
                                        ocr_results.iter().find(|(i, _)| i == page_index)
                                    {
                                        extracted_any_text = true;
                                        rendered_pages.push(sec.clone());
                                    } else {
                                        rendered_pages.push(
                                            "<section class=\"pdf-page\"><p></p></section>"
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        if extracted_any_text && !rendered_pages.is_empty() {
                            let combined = pdf_import::combine_html_pages(&rendered_pages);
                            let styled = page_setup::apply_page_setup(&combined, &page_setup());
                            html.set(styled);
                            chat_history.set(Vec::new());
                            tab.set(Tab::Preview);
                            status.set(Status::Ok(
                                "Converted scanned PDF to editable HTML via fast local OCR. Switch to Visual Edit to edit any element, or use Convert Page (AI) for richer layout styling."
                                    .into(),
                            ));
                            return;
                        }
                    }
                }
            }

            if cfg.provider.is_cloud() {
                if ensure_model_ready(&cfg, status).await {
                    convert_pdf_with_ai(cfg, extra, &pdf, &plan, state).await;
                }
            } else {
                match ollama::is_model_installed(&cfg.ollama_url, &cfg.model).await {
                    Ok(true) => {
                        convert_pdf_with_ai(cfg, extra, &pdf, &plan, state).await;
                    }
                    Ok(false) => {
                        status.set(Status::MissingModel(cfg.model));
                    }
                    Err(_) => {
                        status.set(Status::Error(
                            "This PDF is a scanned image with no selectable text layer. Install Tesseract OCR for fast offline extraction, start Ollama (`ollama serve`), or add a cloud API key in Settings."
                                .into(),
                        ));
                    }
                }
            }
        });
    };

    let convert_pdf_page_ai = move |_| {
        let Some(pdf) = pdf_doc() else { return };
        let Some(page) = pdf.current_page().cloned() else {
            status.set(Status::Error(
                "No rendered page image available for AI vision conversion. Use Quick Extract HTML instead.".into(),
            ));
            return;
        };
        let cfg = settings();
        let extra = extra_prompt();
        let init_msg = if cfg.provider.is_cloud() {
            format!("Preparing page for {} ({})…", cfg.active_model(), cfg.provider.short_label())
        } else {
            format!("Checking if `{}` is installed in Ollama…", cfg.model)
        };
        status.set(Status::Busy(init_msg));
        start_cancellable_task(active_task, task_seq, async move {
            if ensure_model_ready(&cfg, status).await {
                convert_image_bytes(cfg, extra, page.png_bytes, state).await;
            }
        });
    };

    let send_refinement = move |instruction_override: Option<String>| {
        let active_task = active_task;
        let task_seq = task_seq;
        let mut chat_history = chat_history;
        let mut refine_input = refine_input;
        let mut status = status;
        let mut html = html;
        if busy {
            return;
        }
        let instruction = instruction_override.unwrap_or_else(&*refine_input);
        let trimmed = instruction.trim().to_string();
        if trimmed.is_empty() || !has_html {
            return;
        }
        let cfg = settings();
        let init_msg = if cfg.provider.is_cloud() {
            format!("Refining HTML with {} ({})…", cfg.active_model(), cfg.provider.short_label())
        } else {
            format!("Checking if `{}` is installed in Ollama…", cfg.model)
        };
        status.set(Status::Busy(init_msg));
        start_cancellable_task(active_task, task_seq, async move {
            if !ensure_model_ready(&cfg, status).await {
                return;
            }

            let prior: Vec<String> = chat_history
                .read()
                .iter()
                .filter(|t| t.role == ChatRole::User)
                .map(|t| t.content.clone())
                .collect();

            chat_history.write().push(ChatTurn {
                role: ChatRole::User,
                content: trimmed.clone(),
            });
            if refine_input.read().trim() == trimmed {
                refine_input.set(String::new());
            }

            let model_name = cfg.active_model().to_string();
            status.set(Status::Busy(format!("Refining HTML with {model_name}…")));
            let current_html = html();
            let mut progress_status = status;
            match ollama::refine_html_streaming(&cfg, &current_html, &prior, &trimmed, move |p| {
                progress_status.set(Status::Busy(format_ai_progress("Refining: ", &model_name, &p)));
            })
            .await
            {
                Ok(updated) => {
                    let final_html = page_setup::apply_page_setup(&updated, &page_setup());
                    let lines = final_html.lines().count();
                    html.set(final_html);
                    chat_history.write().push(ChatTurn {
                        role: ChatRole::Assistant,
                        content: format!("Updated document HTML ({lines} lines)."),
                    });
                    status.set(Status::Ok("HTML refined by AI.".into()));
                }
                Err(e) => {
                    chat_history.write().push(ChatTurn {
                        role: ChatRole::Assistant,
                        content: format!("Error: {e:#}"),
                    });
                    status.set(Status::Error(format!("{e:#}")));
                }
            }
        });
    };

    let download_missing_model = move |_| {
        let cfg = settings();
        let extra = extra_prompt();
        status.set(Status::Downloading(PullUpdate {
            text: format!("Starting download of `{}` from Ollama…", cfg.model),
            percent: Some(0),
        }));

        start_cancellable_task(active_task, task_seq, async move {
            let pull_res = ollama::pull_model(&cfg.ollama_url, &cfg.model, |update| {
                status.set(Status::Downloading(update));
            })
            .await;

            match pull_res {
                Ok(()) => match mode() {
                    Mode::ImageToPdf => {
                        if let Some(img) = image() {
                            convert_image_bytes(cfg, extra, img.bytes, state).await;
                        } else {
                            status.set(Status::Ok(format!(
                                "Model `{}` downloaded and ready. Open an image to convert.",
                                cfg.model
                            )));
                        }
                    }
                    Mode::PromptToPdf => {
                        if !doc_prompt.read().trim().is_empty() {
                            generate_from_prompt(cfg, doc_prompt(), state).await;
                        } else {
                            status.set(Status::Ok(format!(
                                "Model `{}` downloaded and ready. Enter a prompt to generate.",
                                cfg.model
                            )));
                        }
                    }
                    Mode::PdfToHtml => {
                        if let Some(pdf) = pdf_doc() {
                            let plan = pdf_import::plan_pdf_extraction(&pdf);
                            if !plan.image_only_pages().is_empty() {
                                convert_pdf_with_ai(cfg, extra, &pdf, &plan, state).await;
                            } else if let Some(page) = pdf.current_page().cloned() {
                                convert_image_bytes(cfg, extra, page.png_bytes, state).await;
                            }
                        } else {
                            status.set(Status::Ok(format!(
                                "Model `{}` downloaded and ready. Import a PDF to convert.",
                                cfg.model
                            )));
                        }
                    }
                },
                Err(e) => status.set(Status::Error(format!("{e:#}"))),
            }
        });
    };

    let update_page_setup = move |new_setup: PageSetup| {
        let mut page_setup = page_setup;
        let mut html = html;
        page_setup.set(new_setup);
        let cur = html();
        if !cur.trim().is_empty() {
            html.set(page_setup::apply_page_setup(&cur, &new_setup));
        }
    };

    let default_name = move |ext: &str| {
        match mode() {
            Mode::ImageToPdf => {
                if let Some(stem) =
                    image().and_then(|i| i.path.file_stem().map(|s| s.to_string_lossy().into_owned()))
                {
                    return format!("{stem}.{ext}");
                }
            }
            Mode::PdfToHtml => {
                if let Some(stem) =
                    pdf_doc().and_then(|p| p.path.file_stem().map(|s| s.to_string_lossy().into_owned()))
                {
                    return format!("{stem}_edited.{ext}");
                }
            }
            Mode::PromptToPdf => {}
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
        let final_html = page_setup::apply_page_setup(&html(), &page_setup());
        start_cancellable_task(active_task, task_seq, async move {
            match pdf::html_to_pdf(&chromium, &final_html, &path).await {
                Ok(()) => status.set(Status::Ok(format!("Saved {}", path.display()))),
                Err(e) => status.set(Status::Error(format!("{e:#}"))),
            }
        });
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
        let final_html = page_setup::apply_page_setup(&html(), &page_setup());
        match tokio::fs::write(&path, final_html).await {
            Ok(()) => status.set(Status::Ok(format!("Saved {}", path.display()))),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    };

    let clear_image = move |_| {
        image.set(None);
        crop_mode.set(false);
        straighten_deg.set(0.0);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let clear_pdf = move |_| {
        pdf_doc.set(None);
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
        chat_history.set(Vec::new());
        refine_open.set(false);
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let clear_all = move |_| {
        match mode() {
            Mode::ImageToPdf => {
                image.set(None);
                crop_mode.set(false);
                straighten_deg.set(0.0);
            }
            Mode::PromptToPdf => doc_prompt.set(String::new()),
            Mode::PdfToHtml => pdf_doc.set(None),
        }
        html.set(String::new());
        chat_history.set(Vec::new());
        refine_open.set(false);
        tab.set(Tab::Preview);
        if !matches!(status(), Status::MissingModel(_)) {
            status.set(Status::Idle);
        }
    };

    let html_line_count = html.read().lines().count();
    let cur_setup = page_setup();
    let (cx, cy, cw, ch) = crop_rect();

    // --- UI ----------------------------------------------------------------

    rsx! {
        div {
            class: "workbench-root",
            ondragenter: move |evt| {
                evt.prevent_default();
                if !busy {
                    drag_active.set(true);
                }
            },
            ondragover: move |evt| {
                evt.prevent_default();
            },
            ondrop: handle_drop,

            if drag_active() && !busy {
                div {
                    class: "drop-overlay",
                    ondragover: move |evt| evt.prevent_default(),
                    ondragleave: move |_| drag_active.set(false),
                    ondrop: handle_drop,
                    div { class: "drop-card",
                        div { class: "empty-icon", aria_hidden: "true",
                            svg {
                                width: "22",
                                height: "22",
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
                        }
                        span { class: "empty-title", "Drop image or PDF here" }
                        span { class: "empty-desc", "Supports PNG, JPG, WEBP, TIFF, BMP, or PDF documents" }
                    }
                }
            }

            div { class: "toolbar", role: "toolbar", aria_label: "Workbench actions",
                div { class: "toolbar-group",
                    div { class: "mode-dropdown",
                        button {
                            class: "mode-select",
                            disabled: busy,
                            aria_haspopup: "listbox",
                            aria_expanded: "{mode_menu_open()}",
                            onclick: move |_| {
                                page_menu_open.set(false);
                                mode_menu_open.toggle();
                            },
                            span {
                                match mode() {
                                    Mode::ImageToPdf => "Image to PDF",
                                    Mode::PromptToPdf => "Prompt to PDF",
                                    Mode::PdfToHtml => "PDF to Editable HTML",
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
                                button {
                                    class: if mode() == Mode::PdfToHtml { "mode-option active" } else { "mode-option" },
                                    role: "option",
                                    aria_selected: "{mode() == Mode::PdfToHtml}",
                                    onclick: move |_| {
                                        mode.set(Mode::PdfToHtml);
                                        mode_menu_open.set(false);
                                    },
                                    span { "PDF to Editable HTML" }
                                    if mode() == Mode::PdfToHtml {
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
                            disabled: busy,
                            title: "Paste image from clipboard (Ctrl+V)",
                            onclick: paste_from_clipboard,
                            "Paste"
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
                    } else if mode() == Mode::PromptToPdf {
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
                    } else {
                        button { disabled: busy, onclick: pick_pdf,
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
                            }
                            "Open PDF"
                        }
                        if is_scanned_pdf {
                            button {
                                class: "primary",
                                disabled: busy || !has_pdf,
                                title: "Recreate the selected PDF page with full layout, styling, and editable elements using local Vision AI",
                                onclick: convert_pdf_page_ai,
                                "Convert Page (AI)"
                            }
                            button {
                                disabled: busy || !has_pdf,
                                title: "Instant offline plain-text & table OCR extraction (no visual styling)",
                                onclick: quick_extract_pdf,
                                if pdf_import::has_tesseract() {
                                    "Fast OCR (Text)"
                                } else {
                                    "Quick Extract HTML"
                                }
                            }
                        } else {
                            button {
                                class: "primary",
                                disabled: busy || !has_pdf,
                                title: "Extract text, headings, and tables directly into editable HTML",
                                onclick: quick_extract_pdf,
                                "Quick Extract HTML"
                            }
                            button {
                                disabled: busy || !has_pdf,
                                title: "Recreate the selected PDF page with full visual layout & styling using local Vision AI",
                                onclick: convert_pdf_page_ai,
                                "Convert Page (AI)"
                            }
                        }
                        button {
                            class: "ghost btn-danger",
                            disabled: busy || (!has_pdf && !has_html),
                            onclick: clear_all,
                            "Clear all"
                        }
                    }
                }

                div { class: "toolbar-group",
                    div { class: "mode-dropdown",
                        button {
                            class: "page-setup-btn",
                            disabled: busy,
                            title: "Configure PDF paper size, orientation, and margins",
                            aria_haspopup: "dialog",
                            aria_expanded: "{page_menu_open()}",
                            onclick: move |_| {
                                mode_menu_open.set(false);
                                page_menu_open.toggle();
                            },
                            svg {
                                class: "icon",
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.9",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                rect { x: "5", y: "3", width: "14", height: "18", rx: "2" }
                                line { x1: "9", y1: "8", x2: "15", y2: "8" }
                                line { x1: "9", y1: "12", x2: "15", y2: "12" }
                            }
                            span { "{cur_setup.summary()}" }
                            span { class: "mode-caret", aria_hidden: "true", "▾" }
                        }
                        if page_menu_open() && !busy {
                            div {
                                class: "mode-backdrop",
                                onclick: move |_| page_menu_open.set(false),
                            }
                            div { class: "page-setup-menu", role: "dialog", aria_label: "Page setup controls",
                                div { class: "page-setup-section",
                                    span { class: "page-setup-label", "Paper Size" }
                                    div { class: "page-setup-pills",
                                        for paper in PaperSize::ALL {
                                            button {
                                                class: if cur_setup.paper == paper { "chip active" } else { "chip" },
                                                onclick: move |_| update_page_setup(PageSetup { paper, ..cur_setup }),
                                                "{paper.label()}"
                                            }
                                        }
                                    }
                                }
                                div { class: "page-setup-section",
                                    span { class: "page-setup-label", "Orientation" }
                                    div { class: "page-setup-pills",
                                        for orient in [Orientation::Portrait, Orientation::Landscape] {
                                            button {
                                                class: if cur_setup.orientation == orient { "chip active" } else { "chip" },
                                                onclick: move |_| update_page_setup(PageSetup { orientation: orient, ..cur_setup }),
                                                "{orient.label()}"
                                            }
                                        }
                                    }
                                }
                                div { class: "page-setup-section",
                                    span { class: "page-setup-label", "Margins" }
                                    div { class: "page-setup-pills",
                                        for (mm, lbl) in [(0u32, "0mm"), (8, "8mm"), (15, "15mm"), (25, "25mm")] {
                                            button {
                                                class: if cur_setup.margin_mm == mm { "chip active" } else { "chip" },
                                                onclick: move |_| update_page_setup(PageSetup { margin_mm: mm, ..cur_setup }),
                                                "{lbl}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

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
                    if busy && active_task().is_some() {
                        button {
                            class: "btn-stop",
                            title: "Stop the current generation or export",
                            onclick: cancel_generation,
                            svg {
                                class: "icon",
                                view_box: "0 0 24 24",
                                fill: "currentColor",
                                rect { x: "6", y: "6", width: "12", height: "12", rx: "2" }
                            }
                            "Stop"
                        }
                    }
                }
            }

            if mode() == Mode::ImageToPdf || mode() == Mode::PdfToHtml {
                div { class: "custom-prompt-box",
                    div { class: "custom-prompt-header",
                        label { r#for: "custom-instructions-input", "Custom instructions" }
                        span { class: "muted", "(appended to AI vision prompt)" }
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
                        span { "{msg}" }
                        if active_task().is_some() {
                            span { class: "spacer" }
                            button {
                                class: "btn-small btn-stop",
                                title: "Cancel current generation",
                                onclick: cancel_generation,
                                svg {
                                    class: "icon",
                                    view_box: "0 0 24 24",
                                    fill: "currentColor",
                                    rect { x: "6", y: "6", width: "12", height: "12", rx: "2" }
                                }
                                "Stop"
                            }
                        }
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
                                Mode::PdfToHtml if has_pdf => format!("Download `{model}` & Convert"),
                                _ => format!("Download `{model}`"),
                            }
                        }
                    }
                },
                Status::MissingApiKey(provider) => rsx! {
                    div { class: "status warn", role: "status", aria_live: "polite",
                        span { "{provider} API key is not configured. Add an API key in Settings for fast cloud execution." }
                        span { class: "spacer" }
                        Link {
                            to: Route::SettingsPage {},
                            class: "primary",
                            style: "text-decoration: none; display: inline-flex; align-items: center; padding: 6px 12px; border-radius: var(--radius-md); font-size: 13px; font-weight: 500;",
                            "Configure API Key"
                        }
                    }
                },
                Status::Downloading(update) => rsx! {
                    div { class: "status busy status-col", role: "status", aria_live: "polite",
                        div { class: "status-row",
                            span { class: "spinner" }
                            span { "{update.text}" }
                            if active_task().is_some() {
                                span { class: "spacer" }
                                button {
                                    class: "btn-small btn-stop",
                                    title: "Cancel download & generation",
                                    onclick: cancel_generation,
                                    svg {
                                        class: "icon",
                                        view_box: "0 0 24 24",
                                        fill: "currentColor",
                                        rect { x: "6", y: "6", width: "12", height: "12", rx: "2" }
                                    }
                                    "Stop"
                                }
                            }
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
                // Left pane: source image (with crop/rotate/deskew) OR prompt composer OR PDF preview
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
                        if let Some(img) = image.read().as_ref() {
                            div { class: "image-tools-bar", role: "toolbar", aria_label: "Image crop, rotate and deskew tools",
                                button {
                                    class: "btn-small",
                                    disabled: busy,
                                    title: "Rotate 90° counter-clockwise",
                                    onclick: rotate_ccw,
                                    "⟲ 90°"
                                }
                                button {
                                    class: "btn-small",
                                    disabled: busy,
                                    title: "Rotate 90° clockwise",
                                    onclick: rotate_cw,
                                    "⟳ 90°"
                                }
                                button {
                                    class: "btn-small",
                                    disabled: busy,
                                    title: "Automatically detect tilt and straighten document lines",
                                    onclick: auto_deskew,
                                    "Auto Deskew"
                                }
                                label { class: "prop-field", title: "Fine straighten angle (-15° to +15°)",
                                    span { class: "muted", "Angle" }
                                    input {
                                        class: "prop-num",
                                        r#type: "number",
                                        min: "-15",
                                        max: "15",
                                        step: "0.5",
                                        disabled: busy,
                                        value: "{straighten_deg}",
                                        oninput: move |e| {
                                            if let Ok(v) = e.value().parse::<f32>() {
                                                straighten_deg.set(v.clamp(-15.0, 15.0));
                                            }
                                        },
                                    }
                                    button {
                                        class: "btn-small",
                                        disabled: busy || straighten_deg().abs() < 0.1,
                                        onclick: apply_straighten,
                                        "Apply"
                                    }
                                }
                                span { class: "editor-sep", aria_hidden: "true" }
                                if !crop_mode() {
                                    button {
                                        class: "btn-small",
                                        disabled: busy,
                                        title: "Show adjustable white crop box on the image",
                                        onclick: move |_| crop_mode.set(true),
                                        "Crop"
                                    }
                                } else {
                                    button {
                                        class: "btn-small primary",
                                        disabled: busy,
                                        onclick: apply_crop,
                                        "Apply Crop"
                                    }
                                    button {
                                        class: "btn-small",
                                        disabled: busy,
                                        onclick: move |_| crop_mode.set(false),
                                        "Cancel"
                                    }
                                }
                                if img.modified {
                                    span { class: "spacer" }
                                    button {
                                        class: "btn-small ghost",
                                        disabled: busy,
                                        title: "Restore original unedited image",
                                        onclick: reset_image_edits,
                                        "Reset"
                                    }
                                }
                            }
                        }
                        div { class: "pane-body image-body",
                            if let Some(img) = image.read().as_ref() {
                                if crop_mode() {
                                    div { class: "crop-container",
                                        div { id: "df-crop-stage", class: "crop-stage",
                                            img { src: "{img.preview}", alt: "Source document crop preview", draggable: false }
                                            div {
                                                class: "crop-box",
                                                style: "left:{cx * 100.0}%;top:{cy * 100.0}%;width:{cw * 100.0}%;height:{ch * 100.0}%;",
                                                span { class: "crop-badge",
                                                    "{(cw * 100.0).round()}% × {(ch * 100.0).round()}%"
                                                }
                                                for h in ["nw", "n", "ne", "e", "se", "s", "sw", "w"] {
                                                    span {
                                                        class: "crop-handle",
                                                        "data-crop-h": "{h}",
                                                    }
                                                }
                                            }
                                        }
                                        span { class: "muted", style: "font-size:12px;",
                                            "Drag the white box to move, or drag any corner/edge handle to resize, then click Apply Crop."
                                        }
                                    }
                                } else {
                                    img { src: "{img.preview}", alt: "Uploaded source document", draggable: false }
                                }
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
                                    span { class: "empty-title", "Select, drop, or paste an image" }
                                    span { class: "empty-desc", "Choose a scanned invoice, receipt, or document, drag & drop a file here, or press Ctrl+V." }
                                    span { class: "empty-formats", "PNG · JPG · WEBP · TIFF · BMP · CTRL+V" }
                                }
                            }
                        }
                    }
                } else if mode() == Mode::PromptToPdf {
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
                } else {
                    section { class: "pane", aria_label: "Source PDF document",
                        header {
                            span { class: "pane-header-title", "Source PDF" }
                            if let Some(pdf) = pdf_doc.read().as_ref() {
                                span { class: "muted path-label", "· {pdf.path.display()}" }
                            }
                            span { class: "spacer" }
                            if let Some(pdf) = pdf_doc.read().as_ref() {
                                if pdf.page_count() > 1 {
                                    button {
                                        class: "btn-small",
                                        disabled: busy || pdf.active_page == 0,
                                        onclick: move |_| {
                                            if let Some(ref mut p) = *pdf_doc.write() {
                                                p.active_page = p.active_page.saturating_sub(1);
                                            }
                                        },
                                        "‹ Prev"
                                    }
                                    span { class: "pane-badge", "{pdf.active_page + 1} / {pdf.page_count()}" }
                                    button {
                                        class: "btn-small",
                                        disabled: busy || pdf.active_page + 1 >= pdf.page_count(),
                                        onclick: move |_| {
                                            if let Some(ref mut p) = *pdf_doc.write() {
                                                if p.active_page + 1 < p.page_count() {
                                                    p.active_page += 1;
                                                }
                                            }
                                        },
                                        "Next ›"
                                    }
                                }
                                button {
                                    class: "btn-small ghost",
                                    disabled: busy,
                                    onclick: clear_pdf,
                                    "Clear"
                                }
                            }
                        }
                        div { class: "pane-body image-body",
                            if let Some(pdf) = pdf_doc.read().as_ref() {
                                if let Some(page) = pdf.current_page() {
                                    img { src: "{page.preview}", alt: "Imported PDF page {page.page_number}", draggable: false }
                                } else {
                                    div { class: "empty-state",
                                        span { class: "empty-title", "PDF loaded (text mode)" }
                                        span { class: "empty-desc", "Preview rasterizer unavailable, but text was extracted. Click Quick Extract HTML to edit it." }
                                    }
                                }
                            } else {
                                button {
                                    class: "empty-card",
                                    disabled: busy,
                                    onclick: pick_pdf,
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
                                            line { x1: "9", y1: "13", x2: "15", y2: "13" }
                                            line { x1: "9", y1: "17", x2: "13", y2: "17" }
                                        }
                                    }
                                    span { class: "empty-title", "Import a PDF document" }
                                    span { class: "empty-desc", "Open or drop a PDF to extract headings, paragraphs, and tables into editable HTML, or convert pages with AI." }
                                    span { class: "empty-formats", "PDF → EDITABLE HTML5" }
                                }
                            }
                        }
                    }
                }

                // Right pane: generated HTML / visual editor + AI Follow-Up Chat
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
                            button {
                                class: if refine_open() { "btn-small active-accent" } else { "btn-small" },
                                disabled: busy,
                                title: "Open AI follow-up chat to refine this document",
                                onclick: move |_| refine_open.toggle(),
                                svg {
                                    class: "icon",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.9",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" }
                                }
                                "Refine with AI"
                            }
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
                                    match mode() {
                                        Mode::ImageToPdf => "Open, drop, or paste an image and click Convert to HTML. Once generated, Visual Edit, AI Refine, and PDF export will unlock.",
                                        Mode::PromptToPdf => "Enter a document prompt and click Generate HTML. Once generated, Visual Edit, AI Refine, and PDF export will unlock.",
                                        Mode::PdfToHtml => "Open or drop a PDF and click Quick Extract HTML or Convert Page (AI). Once generated, Visual Edit and PDF export will unlock.",
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

                    if has_html && refine_open() {
                        div { class: "refine-drawer", role: "region", aria_label: "AI Follow-Up Chat",
                            div { class: "refine-header",
                                span { class: "pane-header-title", "Refine with AI" }
                                div { class: "chips",
                                    for suggestion in [
                                        "Make layout more compact",
                                        "Style tables cleanly",
                                        "Add 10% tax row",
                                        "Fix alignment & spacing",
                                    ] {
                                        button {
                                            class: "chip",
                                            disabled: busy,
                                            onclick: move |_| send_refinement(Some(suggestion.into())),
                                            "{suggestion}"
                                        }
                                    }
                                }
                                span { class: "spacer" }
                                if !chat_history.read().is_empty() {
                                    button {
                                        class: "btn-small ghost",
                                        disabled: busy,
                                        onclick: move |_| chat_history.set(Vec::new()),
                                        "Clear chat"
                                    }
                                }
                                button {
                                    class: "btn-small ghost",
                                    title: "Close AI Refine panel",
                                    onclick: move |_| refine_open.set(false),
                                    "×"
                                }
                            }

                            if !chat_history.read().is_empty() {
                                div { class: "refine-messages",
                                    for turn in chat_history() {
                                        div {
                                            class: if turn.role == ChatRole::User { "refine-msg user" } else { "refine-msg assistant" },
                                            span { class: "refine-role",
                                                if turn.role == ChatRole::User { "You" } else { "AI" }
                                            }
                                            span { class: "refine-text", "{turn.content}" }
                                        }
                                    }
                                }
                            }

                            form {
                                class: "refine-form",
                                onsubmit: move |evt| {
                                    evt.prevent_default();
                                    send_refinement(None)
                                },
                                input {
                                    class: "refine-input",
                                    aria_label: "Follow-up refinement instruction",
                                    disabled: busy,
                                    placeholder: "Ask AI to refine this HTML (e.g. \"Change header accent to dark green\", \"Add a notes footer\")…",
                                    value: "{refine_input}",
                                    oninput: move |e| refine_input.set(e.value()),
                                }
                                if busy && active_task().is_some() {
                                    button {
                                        class: "btn-small btn-stop",
                                        r#type: "button",
                                        title: "Cancel refinement",
                                        onclick: cancel_generation,
                                        svg {
                                            class: "icon",
                                            view_box: "0 0 24 24",
                                            fill: "currentColor",
                                            rect { x: "6", y: "6", width: "12", height: "12", rx: "2" }
                                        }
                                        "Stop"
                                    }
                                } else {
                                    button {
                                        class: "primary btn-small",
                                        r#type: "submit",
                                        disabled: busy || refine_input.read().trim().is_empty(),
                                        "Apply"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
