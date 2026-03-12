use std::time::Duration;

use anyhow::{anyhow, Result};
use async_std::task::sleep;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};

use crate::{
    components::{
        badge::Badge,
        button::Button,
        card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle},
    },
    models::{
        ActiveInfo, ActiveStatus, Device, Endpoint, EndpointTrait, Endpoints, Project, Projects,
    },
    views::global::HeaderContext,
    Route,
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
                        p { "Endpoint: {p.endpoint_key} Not Found" }
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4", {cards} }
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

#[component]
pub fn MonitorProjectSelectPage() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Monitor Devices");
    });

    let projects = use_context::<Signal<Projects>>();

    let projects_clone = projects();
    let cards = projects_clone.iter().map(move |(k, p)| {
        rsx! {
            Link {
                to: Route::MonitorProjectPage {
                    project_name: k.clone(),
                },
                Card {
                    CardHeader {
                        CardTitle { "{k}" }
                    }
                    CardContent {
                        p { "Project key: {p.project_key}" }
                        p { "Endpoint: {p.endpoint_key}" }
                    }
                }
            }
        }
    });

    rsx! {
        h1 { class: "text-2xl font-bold mb-4", "Project Select" }
        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4", {cards} }
    }
}

#[component]
pub fn MonitorProjectPage(project_name: ReadSignal<String>) -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Monitor Devices");
    });
    let projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();
    let project = use_memo(move || projects().get(&project_name()).cloned());
    let endpoint = use_memo(move || {
        if let Some(project) = project() {
            endpoints().get(&project.endpoint_key).cloned()
        } else {
            None
        }
    });
    let project_id = use_memo(move || project().map(|p| p.project_key));
    let project_meta: Resource<Result<Vec<Device>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let project_id = project_id().ok_or_else(|| anyhow!("No project id"))?;
        let endpoint = endpoint().ok_or_else(|| anyhow!("No Endpoint"))?;
        let url = endpoint.metadata();
        let mut data = client
            .get(url)
            .header("CK", project_id.as_str())
            .send()
            .await?
            .json::<Vec<Device>>()
            .await?;
        data.sort_by_key(|v| v.id.parse::<u64>().unwrap_or_default());
        Ok(data)
    });

    rsx! {
        h1 { class: "text-2xl font-bold mb-4", {project_name} }
        p { "🚧施工中🚧" }

        if let Some(resource) = &*project_meta.read() {
            match resource {
                Ok(devices) => rsx! {
                    div { class: "grid sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
                        for d in devices {
                            DeviceMonitorPanel {
                                project: project().unwrap(),
                                endpoint: endpoint().unwrap(),
                                device: d.clone(),
                            }
                        }
                    }
                },
                Err(_) => rsx! {
                    p { "Load error" }
                },
            }
        } else {
            p { "Loading..." }
        }
    }
}

#[component]
pub fn DeviceMonitorPanel(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
) -> Element {
    let mut monitor_res: Resource<Result<Option<ActiveInfo>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let info = client
            .get(endpoint().active(&device().id))
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Option<ActiveInfo>>()
            .await?;
        Ok(info)
    });

    let card_content = if let Some(active_info) = &*monitor_res.read() {
        match active_info {
            Ok(active_info) => {
                if let Some(active_info) = active_info {
                    rsx! {
                        p { "Status: {active_info.status}" }
                        if let Some(record) = active_info.record {
                            p { "Record: {record}" }
                        }
                        if let Some(t) = &active_info.last_data_time {
                            p { "Last: {t}" }
                        }
                    }
                } else {
                    rsx! { "Status: Unset" }
                }
            }
            Err(_) => rsx! { "Load Error" },
        }
    } else {
        rsx! { "Loading" }
    };

    let mut is_animate = use_signal(|| "hidden".to_string());
    let color_dot = if let Some(Ok(Some(active_info))) = &*monitor_res.read() {
        let (class1, class2) = match active_info.status {
            ActiveStatus::Online => ("bg-green-400", "bg-green-500"),
            ActiveStatus::Offline => ("bg-red-400", "bg-red-500"),
            ActiveStatus::Abnormal => ("bg-amber-400", "bg-amber-500"),
            _ => ("bg-gray-400", "bg-gray-300"),
        };

        rsx! {
            span { class: "relative flex size-3",
                span { class: "absolute inline-flex h-full w-full rounded-full  opacity-75 {class1} {is_animate()}" }
                span { class: "relative inline-flex size-3 rounded-full {class2}" }
            }
        }
    } else {
        rsx! {
            span { class: "relative flex size-3",
                span { class: "absolute inline-flex h-full w-full rounded-full opacity-75 bg-gray-400 {is_animate()}" }
                span { class: "relative inline-flex size-3 rounded-full bg-gray-300" }
            }
        }
    };

    use_future(move || async move {
        loop {
            is_animate.set("animate-ping".to_string());
            sleep(Duration::from_secs(5)).await;
            is_animate.set("hidden".to_string());
            sleep(Duration::from_secs(25)).await;
            monitor_res.restart();
        }
    });

    rsx! {
        Card {
            CardHeader {
                CardTitle { {device().name} }
                CardDescription { {device().id} }
                CardAction { {color_dot} }
            }
            CardContent { {card_content} }
        }
    }
}
