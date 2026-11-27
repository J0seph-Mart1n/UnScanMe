//! Settings page: Ollama connection, model, prompt and Chromium path.

use crate::{
    ollama::{self, PullUpdate},
    settings::{AiProvider, Settings, DEFAULT_PROMPT},
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum SettingsBanner {
    Busy(String),
    Downloading(PullUpdate),
    Ok(String),
    Error(String),
}

async fn test_connection_for(
    cfg: Settings,
    mut models: Signal<Vec<String>>,
    mut connected: Signal<bool>,
    mut banner: Signal<Option<SettingsBanner>>,
) {
    let provider_label = cfg.provider.label();
    banner.set(Some(SettingsBanner::Busy(format!(
        "Testing connection to {provider_label}…"
    ))));
    match ollama::test_provider_connection(&cfg).await {
        Ok(list) => {
            let n = list.len();
            models.set(list);
            connected.set(true);
            let msg = if cfg.provider == AiProvider::Ollama {
                format!("Connected to Ollama: {n} model(s) installed.")
            } else {
                format!("Connected to {provider_label}! {n} model(s) available.")
            };
            banner.set(Some(SettingsBanner::Ok(msg)));
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
    let mut show_api_key = use_signal(|| false);

    let dirty = draft() != saved();
    let downloading = matches!(banner(), Some(SettingsBanner::Downloading(_)));
    let active_provider = draft.read().provider;
    let current_model = draft.read().active_model().trim().to_string();
    let is_installed = ollama::model_in_list(&models.read(), &current_model);
    let stored_key = draft.read().active_api_key_stored().to_string();
    let (_, key_from_env) = draft.read().effective_api_key();
    let mut synced_saved_provider = use_signal(|| saved.peek().provider);

    // Sync provider toggles triggered from the top Navbar switch without discarding unsaved key edits.
    use_effect(move || {
        let saved_provider = saved().provider;
        if saved_provider != *synced_saved_provider.peek() {
            synced_saved_provider.set(saved_provider);
            draft.write().set_provider(saved_provider);
            models.set(Vec::new());
            connected.set(false);
            show_api_key.set(false);
        }
    });

    // Auto-check local Ollama on open; cloud API keys are tested on demand.
    use_effect(move || {
        let cfg = draft.peek().clone();
        if cfg.provider == AiProvider::Ollama {
            spawn(test_connection_for(cfg, models, connected, banner));
        }
    });

    let test_connection = move |_| test_connection_for(draft(), models, connected, banner);

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
                    section { class: "settings-card", aria_label: "AI provider and model settings",
                        div { class: "settings-card-header",
                            h2 { class: "settings-card-title", "AI provider & model" }
                            if !active_provider.is_cloud() && connected() && !current_model.is_empty() {
                                if is_installed {
                                    span { class: "badge ok", "Installed" }
                                } else {
                                    span { class: "badge warn", "Not installed" }
                                }
                            } else if active_provider.is_cloud() && connected() {
                                span { class: "badge ok", "API Connected" }
                            } else if active_provider.is_cloud() && stored_key.trim().is_empty() && key_from_env {
                                if let Some(env_name) = active_provider.env_var_name() {
                                    span { class: "badge ok", "Using {env_name}" }
                                }
                            }
                        }
                        div { class: "exec-switch-full", role: "group", aria_label: "Execution mode",
                            button {
                                class: if !active_provider.is_cloud() { "exec-switch-btn active" } else { "exec-switch-btn" },
                                aria_pressed: "{!active_provider.is_cloud()}",
                                disabled: downloading,
                                onclick: move |_| {
                                    draft.write().switch_execution_mode(false);
                                    models.set(Vec::new());
                                    connected.set(false);
                                    show_api_key.set(false);
                                    banner.set(None);
                                },
                                "Ollama (Local)"
                            }
                            button {
                                class: if active_provider.is_cloud() { "exec-switch-btn active" } else { "exec-switch-btn" },
                                aria_pressed: "{active_provider.is_cloud()}",
                                disabled: downloading,
                                onclick: move |_| {
                                    draft.write().switch_execution_mode(true);
                                    models.set(Vec::new());
                                    connected.set(false);
                                    show_api_key.set(false);
                                    banner.set(None);
                                },
                                "API Key (Cloud · Faster)"
                            }
                        }

                        if active_provider.is_cloud() {
                            div { class: "chips", role: "radiogroup", aria_label: "Select cloud API provider",
                                for p in AiProvider::CLOUD {
                                    button {
                                        class: if active_provider == p { "chip active" } else { "chip" },
                                        disabled: downloading,
                                        onclick: move |_| {
                                            draft.write().set_provider(p);
                                            models.set(Vec::new());
                                            connected.set(false);
                                            show_api_key.set(false);
                                            banner.set(None);
                                        },
                                        "{p.label()}"
                                    }
                                }
                            }
                        }

                        if active_provider == AiProvider::Ollama {
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
                                        for &m in active_provider.suggested_models() {
                                            option { value: "{m}" }
                                        }
                                    }
                                    button {
                                        disabled: downloading,
                                        onclick: test_connection,
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
                        } else {
                            span { class: "settings-card-desc",
                                "Cloud API execution runs several times faster than local inference. Keys are stored locally with owner-only (0600) file permissions."
                            }
                            if active_provider == AiProvider::OpenAiCompatible {
                                div { class: "chips",
                                    span { class: "muted", "Presets:" }
                                    for (name, url, model_id) in [
                                        ("OpenRouter", "https://openrouter.ai/api/v1", "google/gemini-2.5-flash"),
                                        ("Groq", "https://api.groq.com/openai/v1", "meta-llama/llama-4-scout-17b-16e-instruct"),
                                        ("Together", "https://api.together.xyz/v1", "meta-llama/Llama-Vision-Free"),
                                    ] {
                                        button {
                                            class: if ollama::normalize_openai_base_url(&draft.read().custom_api_url) == url {
                                                "chip active"
                                            } else {
                                                "chip"
                                            },
                                            onclick: move |_| {
                                                let mut d = draft.write();
                                                d.custom_api_url = url.into();
                                                d.custom_model = model_id.into();
                                                models.set(Vec::new());
                                                connected.set(false);
                                            },
                                            "{name}"
                                        }
                                    }
                                }
                                label { "API Base URL"
                                    input {
                                        placeholder: "https://openrouter.ai/api/v1",
                                        value: "{draft.read().custom_api_url}",
                                        oninput: move |e| {
                                            draft.write().custom_api_url = e.value();
                                            connected.set(false);
                                        },
                                    }
                                }
                            }
                            label { "API Key"
                                div { class: "row",
                                    input {
                                        r#type: if show_api_key() { "text" } else { "password" },
                                        autocomplete: "off",
                                        placeholder: "Paste your {active_provider} API key…",
                                        value: "{stored_key}",
                                        oninput: move |e| {
                                            if let Some(slot) = draft.write().active_api_key_mut() {
                                                *slot = e.value();
                                            }
                                            connected.set(false);
                                        },
                                    }
                                    button {
                                        onclick: move |_| show_api_key.toggle(),
                                        if show_api_key() { "Hide" } else { "Show" }
                                    }
                                    button {
                                        class: "primary",
                                        onclick: test_connection,
                                        "Test Key"
                                    }
                                }
                                if stored_key.trim().is_empty() {
                                    if let Some(env_name) = active_provider.env_var_name() {
                                        span { class: "settings-card-desc",
                                            if key_from_env {
                                                "Detected key from {env_name} environment variable."
                                            } else {
                                                "Or set {env_name} in your environment."
                                            }
                                        }
                                    }
                                }
                            }
                            label { "Model"
                                input {
                                    list: "model-list",
                                    value: "{draft.read().active_model()}",
                                    oninput: move |e| *draft.write().active_model_mut() = e.value(),
                                }
                                datalist { id: "model-list",
                                    for &m in active_provider.suggested_models() {
                                        option { value: "{m}" }
                                    }
                                    for m in models() {
                                        option { value: "{m}" }
                                    }
                                }
                            }
                            div { class: "chips chips-scroll",
                                span { class: "muted", "Suggested:" }
                                for &m in active_provider.suggested_models() {
                                    button {
                                        class: if current_model == m { "chip active" } else { "chip" },
                                        onclick: move |_| *draft.write().active_model_mut() = m.to_string(),
                                        "{m}"
                                    }
                                }
                                for m in models() {
                                    if !active_provider.suggested_models().contains(&m.as_str()) {
                                        button {
                                            class: if current_model == m { "chip active" } else { "chip" },
                                            onclick: {
                                                let m = m.clone();
                                                move |_| *draft.write().active_model_mut() = m.clone()
                                            },
                                            "{m}"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "grid",
                            if active_provider == AiProvider::Ollama {
                                label { "Context window"
                                    input {
                                        r#type: "number", min: "2048", step: "1024",
                                        value: "{draft.read().num_ctx}",
                                        oninput: move |e| if let Ok(v) = e.value().parse() { draft.write().num_ctx = v },
                                    }
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
