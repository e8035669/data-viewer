use std::time::Duration;

use crate::{
    Route, components::{
        button::{Button, ButtonVariant},
        card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle},
        dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
        input::Input,
        label::Label,
        textarea::Textarea,
    }, models::{Device, EditDevice, Endpoint, EndpointTrait, Endpoints, Project, Projects}, views::global::HeaderContext
};
use anyhow::{anyhow, Result};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;

struct ApiHelper;

impl ApiHelper {
    async fn req_project_meta(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
    ) -> Result<Vec<Device>> {
        let url = endpoint.metadata();
        let mut data = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Vec<Device>>()
            .await?;
        data.sort_by_key(|v| v.id.parse::<u64>().unwrap_or_default());
        Ok(data)
    }

    async fn create_device(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        new_device: &EditDevice,
    ) -> Result<String> {
        let url = endpoint.all_device();
        let ret = client
            .post(url)
            .header("CK", project_key)
            .json(new_device)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    async fn delete_device(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        target: &str,
    ) -> Result<()> {
        let url = endpoint.device(target);
        let _ret = client
            .delete(url)
            .header("CK", project_key)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ProjectContext {
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    project_meta: ReadSignal<Vec<Device>>,
}

#[component]
pub fn ProjectLayout(project_name: ReadSignal<String>) -> Element {
    let projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();

    let project = use_memo(move || projects().get(&project_name()).cloned());
    let endpoint =
        use_memo(move || project().map(|p| endpoints().get(&p.endpoint_key).cloned())?);

    let project_meta: Resource<Result<Vec<Device>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let project = project().ok_or_else(|| anyhow!("No Project"))?;
        let project_key = project.project_key;
        let endpoint = endpoint().ok_or_else(|| anyhow!("No Endpoint"))?;
        ApiHelper::req_project_meta(&client, &endpoint, &project_key).await
    });
    use_context_provider(|| project_meta);

    let content = match &*project_meta.read_unchecked() {
        Some(Ok(v)) => {
            if let (Some(p), Some(e)) = (project(), endpoint()) {
                rsx! {
                    ProvideProjectContext { project: p, endpoint: e, project_meta: v.clone(), Outlet::<Route> {} }
                }
            } else {
                rsx! {
                    p { "Unexpected Error" }
                }
            }
        }
        Some(Err(_)) => rsx! {
            p { "Failed to fetch project meta" }
        },
        None => rsx! {
            p { "Loading..." }
        },
    };

    rsx! {
        {content}
    }
}

#[component]
fn ProvideProjectContext(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    project_meta: ReadSignal<Vec<Device>>,
    children: Element,
) -> Element {
    use_context_provider(move || ProjectContext {
        project,
        endpoint,
        project_meta,
    });
    rsx! {
        {children}
    }
}

#[derive(Store)]
pub struct DeleteCtx {
    pub is_open: bool,
    pub target: String,
}

#[store]
impl<Lens> Store<DeleteCtx, Lens> {
    fn prompt_delete(&mut self, target: &str) {
        self.target().set(target.to_string());
        self.is_open().set(true);
    }
}

#[component]
pub fn ProjectDevices(project_name: ReadSignal<String>) -> Element {
    use_effect(move || {
        consume_context::<HeaderContext>().set_title(&project_name());
    });

    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();
    let toast_api = use_toast();

    let mut new_device = use_signal(|| EditDevice::new());
    let mut new_device_open = use_signal(|| false);
    let on_create_btn = move |_| async move {
        new_device_open.set(false);
        let client = Client::new();
        let endpoint = endpoint();
        let project = project();
        let ret =
            ApiHelper::create_device(&client, &endpoint, &project.project_key, &new_device()).await;
        match ret {
            Ok(ret) => {
                toast_api.success(
                    "Create Success".to_string(),
                    ToastOptions::new()
                        .description(ret.as_str())
                        .duration(Duration::from_secs(10)),
                );
            }
            Err(e) => {
                toast_api.error(
                    "Create Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{:?}", e))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
        project_resource.restart();
    };

    let dialog = rsx! {
        DialogRoot {
            open: new_device_open(),
            on_open_change: move |v| new_device_open.set(v),
            DialogContent {
                DialogTitle { "New Device" }
                div { class: "grid grid-cols-1 gap-4",
                    Label { html_for: "add_device_name", "Name" }
                    Input {
                        id: "add_device_name",
                        value: new_device().name,
                        oninput: move |e: FormEvent| new_device.write().name = e.value(),
                    }
                    Label { html_for: "add_device_desc", "Description" }
                    Textarea {
                        id: "add_device_desc",
                        value: new_device().desc,
                        oninput: move |e: FormEvent| new_device.write().desc = Some(e.value()),
                    }
                    Button { onclick: on_create_btn, "Create" }
                }
            }
        }
    };

    let mut delete_ctx = use_store(|| DeleteCtx {
        is_open: false,
        target: String::new(),
    });
    let on_delete_confirm = move |_| async move {
        let toast_api = use_toast();
        let target = delete_ctx.target()();
        delete_ctx.write().is_open = false;
        let project = project();
        let endpoint = endpoint();
        let client = Client::new();
        let ret =
            ApiHelper::delete_device(&client, &endpoint, &project.project_key, target.as_str())
                .await;
        match ret {
            Ok(()) => {
                toast_api.success(
                    "Delete Success".to_string(),
                    ToastOptions::new().duration(Duration::from_secs(10)),
                );
            }
            Err(e) => {
                toast_api.error(
                    "Delete Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{:?}", e))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
        project_resource.restart();
    };
    let delete_dialog = rsx! {
        DialogRoot {
            open: *delete_ctx.is_open().read(),
            on_open_change: move |v| delete_ctx.is_open().set(v),
            DialogContent {
                DialogTitle { "Delete Device" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        "Delete device {delete_ctx.target()}"
                        div { class: "flex justify-end gap-4",
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_ctx.is_open().set(false),
                                "NO"
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: on_delete_confirm,
                                "Yes"
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        h1 { class: "text-2xl mb-4", "Devices" }
        div { class: "flex justify-end gap-4 mb-4",
            Button {
                onclick: move |_| {
                    new_device.set(EditDevice::new());
                    new_device_open.set(true)
                },
                "New Device"
            }
        }
        {dialog}
        {delete_dialog}
        div { class: "grid sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 items-start",
            for d in project_meta() {
                DeviceCard { key: "{d.id}", device: d.clone(), delete_ctx }
            }
        }
        div { class: "h-96" }
    }
}

#[component]
pub fn DeviceCard(device: ReadSignal<Device>, delete_ctx: Store<DeleteCtx>) -> Element {
    let desc = device().desc.unwrap_or_default();
    let view_sensor = move |_| {
        // ctx.view_sensors(&device().id)
    };
    let view_device_attr = move |_| {
        // e.stop_propagation();
        // ctx.view_device_attr(&device().id)
    };
    let on_delete = move |_| {
        delete_ctx.prompt_delete(&device().id);
    };
    rsx! {
        div { class: "cursor-pointer", onclick: view_sensor,
            Card {
                CardHeader {
                    CardTitle { {device().name} }
                    CardDescription { {desc} }
                    CardAction {
                        DropdownMenu {
                            DropdownMenuTrigger {
                                r#as: |attributes| rsx! {
                                    Button {
                                        attributes,
                                        onclick: |e: Event<MouseData>| e.stop_propagation(),
                                        variant: ButtonVariant::Ghost,
                                        Icon { icon: fa_solid_icons::FaEllipsisVertical }
                                    }
                                },
                            }
                            DropdownMenuContent {
                                DropdownMenuItem::<String> {
                                    index: 0usize,
                                    value: "Setting".to_string(),
                                    on_select: view_device_attr,
                                    div { class: "flex gap-2",
                                        Icon { icon: fa_solid_icons::FaGear }
                                        "Setting"
                                    }
                                }
                                DropdownMenuItem::<String> {
                                    index: 1usize,
                                    value: "Delete".to_string(),
                                    on_select: on_delete,
                                    div { class: "flex gap-2",
                                        Icon { icon: fa_solid_icons::FaTrash }
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                }
                CardContent {
                    p { {device().id} }
                }
            }
        }
    }
}
