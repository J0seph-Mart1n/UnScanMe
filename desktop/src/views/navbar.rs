use crate::{
    settings::Settings,
    views::{ConvertState, Status},
    Route,
};
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let state = use_context::<ConvertState>();
    let busy = matches!((state.status)(), Status::Busy(_) | Status::Downloading(_));
    let provider = settings.read().provider;
    let is_cloud = provider.is_cloud();
    let target_cloud = settings.read().preferred_cloud_provider();
    let model = settings.read().active_model().to_string();
    let badge_label = if is_cloud {
        provider.short_label().to_uppercase()
    } else {
        "LOCAL AI".to_string()
    };
    let pill_label = if is_cloud {
        format!("{} · {}", provider.short_label(), model)
    } else {
        model
    };

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        nav { id: "navbar", aria_label: "Main navigation",
            div { class: "nav-brand",
                span { class: "nav-emblem", aria_hidden: "true",
                    svg {
                        width: "15",
                        height: "15",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2.2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                        polyline { points: "14 2 14 8 20 8" }
                        line { x1: "9", y1: "15", x2: "15", y2: "15" }
                    }
                }
                span { "UnScanMe" }
                span { class: "nav-badge", "{badge_label}" }
            }

            div { class: "nav-tabs",
                Link { to: Route::Convert {}, active_class: "active", "Workbench" }
                Link { to: Route::SettingsPage {}, active_class: "active", "Settings" }
            }

            div { class: "nav-status",
                div { class: "exec-switch", role: "group", aria_label: "AI execution mode",
                    button {
                        class: if !is_cloud { "exec-switch-btn active" } else { "exec-switch-btn" },
                        aria_pressed: "{!is_cloud}",
                        disabled: busy,
                        title: "Run locally using Ollama",
                        onclick: move |_| {
                            settings.write().switch_execution_mode(false);
                            let _ = settings.read().save();
                        },
                        "Ollama"
                    }
                    button {
                        class: if is_cloud { "exec-switch-btn active" } else { "exec-switch-btn" },
                        aria_pressed: "{is_cloud}",
                        disabled: busy,
                        title: "Run faster using {target_cloud} API key",
                        onclick: move |_| {
                            settings.write().switch_execution_mode(true);
                            let _ = settings.read().save();
                        },
                        "API Key"
                    }
                }
                Link {
                    to: Route::SettingsPage {},
                    class: "nav-model-pill",
                    style: "padding: 4px 10px; border-radius: 999px; font-size: 12px; background: var(--bg-panel);",
                    title: "Click to change AI provider, API key, or model",
                    span { class: "nav-dot", aria_hidden: "true" }
                    "{pill_label}"
                }
            }
        }

        main { id: "content", Outlet::<Route> {} }
    }
}
