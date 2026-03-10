use anyhow::Result;
use dioxus::prelude::*;

use crate::{
    components::{
        badge::Badge,
        card::{Card, CardContent, CardHeader, CardTitle},
    },
    models::{Endpoint, Endpoints, Project, Projects},
    views::global::HeaderContext,
};

#[component]
pub fn ActiveMonitorView() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Active Monitor");
    });

    let mut projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();

    let projects_clone = projects();
    let cards = projects_clone.iter().map(move |(k, p)| {
        let endpoints = endpoints();
        let endpoint = endpoints.get(&p.endpoint_key);
        if let Some(endpoint) = endpoint {
            rsx! {
                Card {
                    CardHeader {
                        CardTitle { "{k}" }
                    }
                    CardContent {
                        ProjectMonitor { project: p.clone(), endpoint: endpoint.clone() }
                    }
                }
            }
        } else {
            rsx! {
                Card {
                    CardHeader {
                        CardTitle { "{k}" }
                    }
                    CardContent {
                        EndpointNotFound { project: p.clone() }
                    }
                }
            }
        }
    });

    rsx! {
        p { "🚧施工中🚧" }
        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4", {cards} }
    }
}

#[component]
pub fn EndpointNotFound(project: ReadSignal<Project>) -> Element {
    rsx! {
        p { "Endpoint: {project().endpoint_key} Not Found" }
    }
}

#[derive(Default, Clone, Copy)]
struct MonitorStatus {
    total: i32,
    online: i32,
    offline: i32,
    abnormal: i32,
    unset: i32,
}

#[component]
pub fn ProjectMonitor(project: ReadSignal<Project>, endpoint: ReadSignal<Endpoint>) -> Element {
    let status_res: Resource<Result<MonitorStatus>> =
        use_resource(move || async move { Ok(MonitorStatus::default()) });

    rsx! {
        div { class: "grid justify-center",
            if let Some(status) = &*status_res.read() {
                match status {
                    Ok(status) => rsx! {
                        div { class: "flex flex-wrap items-center gap-2",
                            div { class: "p-2 outline", "總數 {status.total}" }
                            div { class: "p-2 outline", "連線 {status.online}" }
                            div { class: "p-2 outline", "斷線 {status.offline}" }
                            div { class: "p-2 outline ", "異常 {status.abnormal}" }
                            div { class: "p-2 outline", "未知 {status.unset}" }
                        }
                    },
                    Err(_) => rsx! { "Load Error" },
                }
            } else {
                "Loading"
            }
        
        }
    }
}
