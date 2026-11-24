use crate::{settings::Settings, Route};
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let settings = use_context::<Signal<Settings>>();
    let model = settings.read().model.clone();

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
                span { "i hate scans" }
                span { class: "nav-badge", "LOCAL AI" }
            }

            div { class: "nav-tabs",
                Link { to: Route::Convert {}, active_class: "active", "Workbench" }
                Link { to: Route::SettingsPage {}, active_class: "active", "Settings" }
            }

            div { class: "nav-status",
                span { class: "nav-model-pill", title: "Active Ollama model",
                    span { class: "nav-dot", aria_hidden: "true" }
                    "{model}"
                }
            }
        }

        main { id: "content", Outlet::<Route> {} }
    }
}
