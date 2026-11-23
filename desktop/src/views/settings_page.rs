//! Settings page: Ollama connection, model, prompt and Chromium path.

use crate::{
    ollama::{self, PullUpdate},
    settings::{Settings, DEFAULT_PROMPT},
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum SettingsBanner {
    Busy(String),
    Downloading(PullUpdate),
    Ok(String),
    Error(String),
}

async fn check_connection(
    url: String,
    mut models: Signal<Vec<String>>,
    mut connected: Signal<bool>,
    mut banner: Signal<Option<SettingsBanner>>,
) {
    banner.set(Some(SettingsBanner::Busy("Checking connection to Ollama…".into())));
    match ollama::list_models(&url).await {
        Ok(list) => {
            let n = list.len();
            models.set(list);
            connected.set(true);
            banner.set(Some(SettingsBanner::Ok(format!(
                "Connected to Ollama: {n} model(s) installed."
            ))));
        }
        Err(e) => {
            connected.set(false);
            banner.set(Some(SettingsBanner::Error(format!("{e:#}"))));
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let mut saved = use_context::<Signal<Settings>>();
    // Edit a local copy; only commit on "Save".
    let mut draft = use_signal(|| saved.read().clone());
    let mut banner = use_signal(|| None::<SettingsBanner>);
    let mut models = use_signal(Vec::<String>::new);
    let mut connected = use_signal(|| false);

    let dirty = draft() != saved();
    let downloading = matches!(banner(), Some(SettingsBanner::Downloading(_)));
    let current_model = draft.read().model.trim().to_string();
    let is_installed = ollama::model_in_list(&models.read(), &current_model);

    // Fetch installed models and update the top status banner when opening Settings.
    use_effect(move || {
        let url = draft.peek().ollama_url.clone();
        spawn(check_connection(url, models, connected, banner));
    });

    let check_ollama = move |_| {
        let url = draft.read().ollama_url.clone();
        check_connection(url, models, connected, banner)
    };

    let download_model = move |_| async move {
        let url = draft.read().ollama_url.clone();
        let model = draft.read().model.trim().to_string();
        if model.is_empty() {
            banner.set(Some(SettingsBanner::Error("Enter a model name first.".into())));
            return;
        }

        banner.set(Some(SettingsBanner::Downloading(PullUpdate {
            text: format!("Starting download of `{model}` from Ollama…"),
            percent: Some(0),
        })));

        let res = ollama::pull_model(&url, &model, |update| {
            banner.set(Some(SettingsBanner::Downloading(update)));
        })
        .await;

        match res {
            Ok(()) => {
                if let Ok(list) = ollama::list_models(&url).await {
                    models.set(list);
                    connected.set(true);
                }
                banner.set(Some(SettingsBanner::Ok(format!(
                    "Model `{model}` downloaded and ready!"
                ))));
            }
            Err(e) => banner.set(Some(SettingsBanner::Error(format!("{e:#}")))),
        }
    };

    let save = move |_| {
        let s = draft();
        match s.save() {
            Ok(path) => {
                saved.set(s);
                banner.set(Some(SettingsBanner::Ok(format!("Saved to {}", path.display()))));
            }
            Err(e) => banner.set(Some(SettingsBanner::Error(format!("{e:#}")))),
        }
    };

    rsx! {
        div { class: "settings",
            div { class: "settings-header",
                div { class: "settings-title-group",
                    h1 { "Settings" }
                    if dirty {
                        span { class: "badge warn", "Unsaved changes" }
                    }
                }
                div { class: "toolbar-group",
                    button { class: "primary", disabled: !dirty || downloading, onclick: save, "Save settings" }
                    button { disabled: !dirty || downloading, onclick: move |_| draft.set(saved()), "Discard" }
                    button { class: "ghost", disabled: downloading, onclick: move |_| draft.set(Settings::default()), "Restore defaults" }
                }
            }

            match banner() {
                Some(SettingsBanner::Busy(m)) => rsx! {
                    div { class: "status busy", role: "status", aria_live: "polite",
                        span { class: "spinner" }
                        "{m}"
                    }
                },
                Some(SettingsBanner::Downloading(update)) => rsx! {
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
                Some(SettingsBanner::Ok(m)) => rsx! {
                    div { class: "status ok", role: "status", aria_live: "polite", "{m}" }
                },
                Some(SettingsBanner::Error(m)) => rsx! {
                    div { class: "status error", role: "alert", "{m}" }
                },
                None => rsx! {},
            }

            div { class: "settings-columns",
                div { class: "settings-col",
                    section { class: "settings-card", aria_label: "Local AI model settings",
                        div { class: "settings-card-header",
                            h2 { class: "settings-card-title", "Local AI model (Ollama)" }
                            if connected() && !current_model.is_empty() {
                                if is_installed {
                                    span { class: "badge ok", "Installed" }
                                } else {
                                    span { class: "badge warn", "Not installed" }
                                }
                            }
                        }
                        label { "Server URL"
                            input {
                                disabled: downloading,
                                value: "{draft.read().ollama_url}",
                                oninput: move |e| draft.write().ollama_url = e.value(),
                            }
                        }
                        label { "Model"
                            div { class: "row",
                                input {
                                    list: "model-list",
                                    disabled: downloading,
                                    value: "{draft.read().model}",
                                    oninput: move |e| draft.write().model = e.value(),
                                }
                                datalist { id: "model-list",
                                    for m in models() {
                                        option { value: "{m}" }
                                    }
                                }
                                button {
                                    disabled: downloading,
                                    onclick: check_ollama,
                                    "Check connection"
                                }
                                button {
                                    class: if !is_installed { "primary" } else { "" },
                                    disabled: downloading || current_model.is_empty(),
                                    onclick: download_model,
                                    if is_installed {
                                        "Re-pull"
                                    } else {
                                        "Download"
                                    }
                                }
                            }
                        }
                        if !models.read().is_empty() {
                            div { class: "chips chips-scroll",
                                span { class: "muted", "Installed:" }
                                for m in models() {
                                    button {
                                        class: if ollama::model_in_list(std::slice::from_ref(&m), &current_model) {
                                            "chip active"
                                        } else {
                                            "chip"
                                        },
                                        disabled: downloading,
                                        onclick: {
                                            let m = m.clone();
                                            move |_| draft.write().model = m.clone()
                                        },
                                        "{m}"
                                    }
                                }
                            }
                        }
                        div { class: "grid",
                            label { "Context window"
                                input {
                                    r#type: "number", min: "2048", step: "1024",
                                    value: "{draft.read().num_ctx}",
                                    oninput: move |e| if let Ok(v) = e.value().parse() { draft.write().num_ctx = v },
                                }
                            }
                            label { "Temperature"
                                input {
                                    r#type: "number", min: "0", max: "2", step: "0.05",
                                    value: "{draft.read().temperature}",
                                    oninput: move |e| if let Ok(v) = e.value().parse() { draft.write().temperature = v },
                                }
                            }
                            label { "Max image px"
                                input {
                                    r#type: "number", min: "512", step: "128",
                                    value: "{draft.read().max_image_dim}",
                                    oninput: move |e| if let Ok(v) = e.value().parse() { draft.write().max_image_dim = v },
                                }
                            }
                        }
                    }

                    section { class: "settings-card", aria_label: "PDF rendering settings",
                        div { class: "settings-card-header",
                            h2 { class: "settings-card-title", "PDF rendering" }
                            span { class: "settings-card-desc", "Headless browser" }
                        }
                        label { "Chromium / Chrome / Edge executable"
                            div { class: "row",
                                input {
                                    value: "{draft.read().chromium_path}",
                                    oninput: move |e| draft.write().chromium_path = e.value(),
                                }
                                button {
                                    onclick: move |_| draft.write().chromium_path = crate::pdf::detect_chromium(),
                                    "Auto-detect"
                                }
                            }
                        }
                    }
                }

                div { class: "settings-col",
                    section { class: "settings-card prompt-card", aria_label: "Base prompt settings",
                        div { class: "settings-card-header",
                            h2 { class: "settings-card-title", "Base prompt" }
                            button {
                                class: "btn-small ghost",
                                disabled: downloading,
                                onclick: move |_| draft.write().prompt = DEFAULT_PROMPT.into(),
                                "Reset prompt"
                            }
                        }
                        textarea {
                            class: "prompt",
                            aria_label: "Base image-to-HTML system prompt",
                            disabled: downloading,
                            value: "{draft.read().prompt}",
                            oninput: move |e| draft.write().prompt = e.value(),
                        }
                    }
                }
            }
        }
    }
}
