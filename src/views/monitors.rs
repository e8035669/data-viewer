use std::time::Duration;

use anyhow::{anyhow, Result};
use async_std::task::sleep;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::checkbox::CheckboxState;

use crate::{
    api::ApiHelper,
    components::{
        card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle},
        checkbox::Checkbox,
    },
    models::{ActiveInfo, ActiveStatus, Device, Endpoint, Endpoints, Project, Projects},
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
        let project_key = project().project_key;
        let devices = ApiHelper::fetch_all_devices(&client, &endpoint(), &project_key).await?;

        let mut status = MonitorStatus::default();
        status.total = devices.len() as i32;

        for device in devices.iter() {
            let info =
                ApiHelper::fetch_active_info(&client, &endpoint(), &device.id, &project_key)
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
    let mut project_meta: Resource<Result<Vec<(Device, Option<ActiveInfo>)>>> =
        use_resource(move || async move {
            let client = reqwest::Client::new();
            let project_id = project_id().ok_or_else(|| anyhow!("No project id"))?;
            let endpoint = endpoint().ok_or_else(|| anyhow!("No Endpoint"))?;
            let data = ApiHelper::req_project_meta(&client, &endpoint, &project_id).await?;

            let mut active_infos = Vec::new();
            for device in data.iter() {
                let info =
                    ApiHelper::fetch_active_info(&client, &endpoint, &device.id, &project_id)
                        .await
                        .map(|info| info.unwrap_or_default());
                active_infos.push(info.ok());
            }

            let device_and_actives = data.into_iter().zip(active_infos).collect::<Vec<_>>();
            Ok(device_and_actives)
        });

    let mut is_loading = use_signal(|| false);

    use_future(move || async move {
        loop {
            is_loading.set(true);
            sleep(Duration::from_secs(5)).await;
            is_loading.set(false);
            sleep(Duration::from_secs(25)).await;
            if project_meta.finished() {
                project_meta.restart();
            }
        }
    });
    let mut is_hide_unset = use_signal(|| CheckboxState::Unchecked);
    let need_show = move |a: &Option<ActiveInfo>| -> bool {
        if is_hide_unset() == CheckboxState::Unchecked {
            true
        } else {
            if let Some(a) = a {
                match a.status {
                    ActiveStatus::Start => true,
                    ActiveStatus::Online => true,
                    ActiveStatus::Offline => true,
                    ActiveStatus::Abnormal => true,
                    _ => false,
                }
            } else {
                true
            }
        }
    };

    let load_cls = if is_loading() { "" } else { "hidden" };

    rsx! {
        h1 { class: "text-2xl font-bold mb-4", {project_name} }

        div { class: "flex justify-end gap-4 min-h-8",
            div { class: "flex items-center gap-2 {load_cls}",
                Icon { class: "animate-spin", icon: fa_solid_icons::FaSpinner }
                "Loading"
            }

            div { class: "flex items-center gap-2",
                Checkbox {
                    checked: is_hide_unset(),
                    on_checked_change: move |v| is_hide_unset.set(v),
                }
                "Hide Unset"
            }
        }

        if let Some(resource) = &*project_meta.read() {
            match resource {
                Ok(devices) => rsx! {
                    div { class: "grid sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
                        for (d , a) in devices {
                            if need_show(a) {
                                DeviceMonitorPanel { key: "{d.id}", device: d.clone(), active_info: a.clone() }
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
    device: ReadSignal<Device>,
    active_info: ReadSignal<Option<ActiveInfo>>,
) -> Element {
    let card_content = if let Some(active_info) = active_info() {
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
        rsx! { "Load Error" }
    };

    let color_dot = if let Some(active_info) = active_info() {
        let (class1, class2) = match active_info.status {
            ActiveStatus::Online => ("bg-green-400", "bg-green-500"),
            ActiveStatus::Offline => ("bg-red-400", "bg-red-500"),
            ActiveStatus::Abnormal => ("bg-amber-400", "bg-amber-500"),
            _ => ("hidden", "bg-gray-300"),
        };

        rsx! {
            span { class: "relative flex size-3",
                span { class: "absolute inline-flex h-full w-full rounded-full  opacity-75 animate-ping {class1}" }
                span { class: "relative inline-flex size-3 rounded-full {class2}" }
            }
        }
    } else {
        rsx! {
            span { class: "relative flex size-3",
                span { class: "relative inline-flex size-3 rounded-full bg-gray-300" }
            }
        }
    };

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
