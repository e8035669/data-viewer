use std::time::Duration;

use anyhow::{anyhow, Result};
use async_std::task::sleep;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};

use crate::{
    components::{
        badge::Badge,
        button::Button,
        card::{Card, CardAction, CardContent, CardHeader, CardTitle},
    },
    models::{
        ActiveInfo, ActiveStatus, Device, Endpoint, EndpointTrait, Endpoints, Project, Projects,
    },
    views::global::HeaderContext,
};

#[component]
pub fn ActiveMonitorView() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Active Monitor");
    });

    let projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();

    let projects_clone = projects();
    let cards = projects_clone.iter().map(move |(k, p)| {
        let endpoints = endpoints();
        let endpoint = endpoints.get(&p.endpoint_key);
        if let Some(endpoint) = endpoint {
            rsx! {
                ProjectMonitor {
                    name: k.clone(),
                    project: p.clone(),
                    endpoint: endpoint.clone(),
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
pub fn ProjectMonitor(
    name: ReadSignal<String>,
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
) -> Element {
    let mut is_loading = use_signal(|| false);
    let mut status_res: Resource<Result<MonitorStatus>> = use_resource(move || async move {
        is_loading.set(true);
        let client = reqwest::Client::new();
        let devices = client
            .get(endpoint().all_device())
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Vec<Device>>()
            .await?;

        let mut status = MonitorStatus::default();
        status.total = devices.len() as i32;

        for device in devices.iter() {
            let info = client
                .get(endpoint().active(&device.id))
                .header("CK", project().project_key.as_str())
                .send()
                .await?
                .json::<Option<ActiveInfo>>()
                .await?;
            if let Some(info) = info {
                match info.status {
                    ActiveStatus::Online => status.online += 1,
                    ActiveStatus::Offline => status.offline += 1,
                    ActiveStatus::Abnormal => status.abnormal += 1,
                    _ => status.unset += 1,
                }
            } else {
                status.unset += 1;
            }
        }
        is_loading.set(false);
        Ok(status)
    });

    use_future(move || async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            if status_res.finished() {
                status_res.restart();
            }
        }
    });

    let content = if let Some(status) = &*status_res.read() {
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
        rsx! { "Loading" }
    };

    let load_cls = if is_loading() {
        "animate-spin"
    } else {
        "hidden"
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle { {name} }
                CardAction {
                    Icon { class: load_cls, icon: fa_solid_icons::FaSpinner }
                }
            }
            CardContent {
                div { class: "grid justify-center", {content} }
            }
        }
    }
}
