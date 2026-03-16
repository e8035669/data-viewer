use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use crate::components::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
use crate::components::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
};
use crate::components::input::Input;
use crate::components::select::{
    Select, SelectGroup, SelectItemIndicator, SelectList, SelectOption, SelectTrigger, SelectValue,
};
use crate::components::switch::{Switch, SwitchThumb};
use crate::components::textarea::Textarea;
use crate::views::endpoints;
use crate::views::global::HeaderContext;
use anyhow::{anyhow, Error, Result};
use async_std::task::sleep;
use base64::prelude::*;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons;
use dioxus_free_icons::Icon;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::{Client, Url};
use time::format_description::well_known::Iso8601;
use time::macros::{datetime, format_description, offset};
use time::{Date, OffsetDateTime, UtcOffset};

use crate::models::{
    ActiveDevice, ActiveInfo, ActiveNotify, ActiveNotifySetting, Attribute, Device, EditDevice,
    EditSensor, Endpoint, EndpointTrait, Endpoints, GetRawData, Project, Projects, RawData, Sensor,
    SensorType, SensorWithData,
};

#[component]
pub fn SensorPanel() -> Element {
    rsx! {
        Card {
            CardHeader {
                // CardTitle displays the main heading.
                CardTitle { "Card Title" }
                // CardDescription provides supporting text.
                CardDescription { "Card description goes here." }
                // CardAction positions action elements (e.g., buttons) in the header.
                CardAction {
                    Button { "Action" }
                }
            }
            // CardContent holds the main body content.
            CardContent {
                p { "Main content of the card." }
            }
            // CardFooter contains footer actions or information.
            CardFooter {
                Button { "Submit" }
            }
        }
    }
}

#[component]
pub fn DevicePanel(device: Device) -> Element {
    let desc = device.desc.unwrap_or_default();
    rsx! {
        div { class: "flex outline rounded-lg p-2 m-4 gap-4 items-center",
            p { class: "outline", "{device.id}" }
            div { class: "flex-1",
                div { class: "text-xl", "{device.name}" }
                p { "{desc}" }
            }
            Button { "View" }
        }
    }
}

#[component]
pub fn DevicesPanels2(
    devices: Vec<Device>,
    selected_device: WriteSignal<Option<String>>,
) -> Element {
    rsx! {
        for d in devices {
            DevicePanel2 { device: d.clone(), selected_device }
        }
    }
}

#[component]
pub fn SensorsPanels2(
    device: Option<Device>,
    selected_device: WriteSignal<Option<String>>,
) -> Element {
    let back = move |_| *selected_device.write() = None;

    let back_btn = rsx! {
        Button { variant: ButtonVariant::Ghost, onclick: back,
            Icon { icon: fa_solid_icons::FaArrowLeft }
        }
    };

    let device_card = if let Some(device) = device {
        let desc = device.desc.unwrap_or_default();
        rsx! {
            Card {
                CardHeader {
                    CardTitle { {device.name} }
                    CardDescription { {desc} }
                    CardAction { {back_btn} }
                }
                CardContent {
                    p { {device.id} }
                }
            }

        }
    } else {
        rsx! {
            Card {
                CardHeader {
                    CardTitle { "Error" }
                    CardAction { {back_btn} }
                }
            }
        }
    };

    rsx! {
        {device_card}
    }
}

#[component]
pub fn DevicePanel2(device: Device, selected_device: WriteSignal<Option<String>>) -> Element {
    let device_clone = device.clone();
    let desc = device.desc.unwrap_or_default();
    let view_sensor = move |_| *selected_device.write() = Some(device_clone.id.clone());
    rsx! {
        div { class: " mb-2",
            Card {
                CardHeader {
                    CardTitle { {device.name} }
                    CardDescription { {desc} }
                    CardAction {
                        Button { variant: ButtonVariant::Ghost,

                            Icon { icon: fa_solid_icons::FaSliders }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: view_sensor,
                            Icon { icon: fa_solid_icons::FaEllipsis }
                        }
                    }
                }
                CardContent {
                    p { {device.id} }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewStatus {
    Device,
    DeviceAttr,
    Sensor,
    SensorAttr,
    SensorHistory,
}

#[derive(Debug, Clone, Store, PartialEq)]
pub struct PageContext {
    view_status: ViewStatus,
    selected_device: Option<String>,
    selected_sensor: Option<String>,
}

#[store]
impl<Lens> Store<PageContext, Lens> {
    fn back_to_device(&mut self) {
        self.selected_device().set(None);
        self.selected_sensor().set(None);
        self.view_status().set(ViewStatus::Device);
    }

    fn view_sensors(&mut self, selected_device: &str) {
        self.selected_device()
            .set(Some(selected_device.to_string()));
        self.view_status().set(ViewStatus::Sensor);
    }

    fn view_sensor_attr(&mut self, selected_sensor: &str) {
        self.selected_sensor()
            .set(Some(selected_sensor.to_string()));
        self.view_status().set(ViewStatus::SensorAttr);
    }

    fn back_to_sensors(&mut self) {
        self.selected_sensor().set(None);
        self.view_status().set(ViewStatus::Sensor);
    }

    fn view_device_attr(&mut self, selected_device: &str) {
        self.selected_device()
            .set(Some(selected_device.to_string()));
        self.view_status().set(ViewStatus::DeviceAttr);
    }

    fn view_sensor_history(&mut self, selected_sensor: &str) {
        self.selected_sensor()
            .set(Some(selected_sensor.to_string()));
        self.view_status().set(ViewStatus::SensorHistory);
    }
}

#[component]
pub fn DevicePage3(project_name: ReadSignal<String>) -> Element {
    use_effect(move || {
        let title = project_name();
        consume_context::<HeaderContext>().set_title(title.as_str());
    });
    let projects = use_context::<Signal<Projects>>();
    // let projects = use_project_persistence();
    let endpoints = use_context::<Signal<Endpoints>>();
    // let endpoints = use_endpoints_persistent();
    let ctx = use_store(|| PageContext {
        view_status: ViewStatus::Device,
        selected_device: None,
        selected_sensor: None,
    });

    let project = use_memo(move || {
        ctx.view_status().set(ViewStatus::Device);
        ctx.selected_device().set(None);

        projects().get(&project_name()).cloned()
    });
    let endpoint = use_memo(move || {
        if let Some(project) = project() {
            endpoints().get(&project.endpoint_key).cloned()
        } else {
            None
        }
    });
    // let project = projects().get(&project_name()).cloned();
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

    let device: Memo<Option<Device>> = use_memo(move || {
        let selected_device = ctx.selected_device()().unwrap_or_default();
        if let Some(resource) = &*project_meta.read() {
            if let Ok(devices) = resource {
                devices.iter().find(|d| d.id == selected_device).cloned()
            } else {
                None
            }
        } else {
            None
        }
    });

    let sensor: Memo<Option<Sensor>> = use_memo(move || {
        let selected_sensor = ctx.selected_sensor()().unwrap_or_default();
        if let Some(device) = device() {
            if let Some(sensors) = device.sensors {
                return sensors.iter().find(|s| s.id == selected_sensor).cloned();
            }
        }
        None
    });

    rsx! {
        if let Some(resource) = &*project_meta.read() {
            match resource {
                Ok(devices) => rsx! {
                    match ctx.view_status()() {
                        ViewStatus::Device => rsx! {
                            DevicesPanels3 { devices: devices.to_owned(), ctx }
                        },
                        ViewStatus::DeviceAttr => rsx! {
                            DeviceAttrPanel {
                                project,
                                endpoint,
                                device,
                                ctx,
                                project_meta,
                            }
                        },
                        ViewStatus::Sensor => rsx! {
                            SensorsPanels3 {
                                project,
                                endpoint,
                                device,
                                ctx,
                            }
                        },
                        ViewStatus::SensorAttr => rsx! {
                            SensorAttrPanel {
                                project,
                                endpoint,
                                device,
                                sensor,
                                ctx,
                                project_meta,
                            }
                        },
                        ViewStatus::SensorHistory => rsx! {
                            SensorHistoryPanel {
                                project,
                                endpoint,
                                device,
                                sensor,
                                ctx,
                                project_meta,
                            }
                        },

                    }
                },
                Err(_) => rsx! {
                    p { "Load error" }
                },
            }
        } else {
            p { "Loading..." }
        }
        div { class: "h-48" }
    }
}

#[component]
pub fn DevicesPanels3(devices: Vec<Device>, ctx: Store<PageContext>) -> Element {
    rsx! {
        h1 { class: "text-2xl mb-4", "Devices" }
        div { class: "flex justify-end gap-4 mb-4",
            Button { "Add Device(TODO)" }
        }
        div { class: "grid sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 items-start",
            for d in devices {
                DevicePanel3 { key: "{d.id}", device: d.clone(), ctx }
            }
        }
    }
}

#[component]
pub fn DevicePanel3(device: Device, ctx: Store<PageContext>) -> Element {
    let device_clone = device.clone();
    let device_clone2 = device.clone();
    let desc = device.desc.unwrap_or_default();
    let view_sensor = move |_| ctx.view_sensors(&device_clone.id);
    let view_device_attr = move |_| {
        // e.stop_propagation();
        ctx.view_device_attr(&device_clone2.id)
    };
    rsx! {
        div { onclick: view_sensor,
            Card {
                CardHeader {
                    CardTitle { {device.name} }
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
                                    div { class: "flex gap-2",
                                        Icon { icon: fa_solid_icons::FaTrash }
                                        "Delete(TODO)"
                                    }
                                }
                            }
                        }
                    
                    // Button {
                    //     variant: ButtonVariant::Ghost,
                    //     onclick: view_device_attr,
                    //     Icon { icon: fa_solid_icons::FaGear }
                    // }
                    // Button {
                    //     variant: ButtonVariant::Ghost,
                    //     onclick: view_sensor,
                    //     Icon { icon: fa_solid_icons::FaEllipsis }
                    // }
                    }
                }
                CardContent {
                    p { {device.id} }
                }
            }
        
        }
    }
}

#[component]
pub fn DeviceHeader(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    ctx: Store<PageContext>,
) -> Element {
    let back = move |_| ctx.back_to_device();

    let back_btn = rsx! {
        Button { variant: ButtonVariant::Ghost, onclick: back,
            Icon { icon: fa_solid_icons::FaArrowLeft }
        }
    };

    let device_card = if let Some(device) = device() {
        let desc = device.desc.unwrap_or_default();
        rsx! {
            Card {
                CardHeader {
                    CardTitle { {device.name} }
                    CardDescription { {desc} }
                    CardAction { {back_btn} }
                }
                CardContent {
                    p { {device.id} }
                }
            }

        }
    } else {
        rsx! {
            Card {
                CardHeader {
                    CardTitle { "Error" }
                    CardAction { {back_btn} }
                }
            }
        }
    };

    rsx! {
        {device_card}

    }
}

#[component]
pub fn SensorsPanels3(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    ctx: Store<PageContext>,
) -> Element {
    let sensor_view = if project().is_some() && endpoint().is_some() && device().is_some() {
        let project = project().unwrap();
        let endpoint = endpoint().unwrap();
        let device = device().unwrap();

        rsx! {
            SensorView3 {
                project,
                endpoint,
                device,
                ctx,
            }
        }
    } else {
        rsx! {}
    };

    rsx! {
        DeviceHeader {
            project,
            endpoint,
            device,
            ctx,
        }
        {sensor_view}
    }
}

#[component]
pub fn SensorView3(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    ctx: Store<PageContext>,
) -> Element {
    let mut timer = use_signal(|| 10);
    let mut resource: Resource<Result<_, Error>> = use_resource(move || async move {
        let device_id = device().id;
        let project_id = project().project_key;
        let client = reqwest::Client::new();

        let sensors = device().sensors.unwrap_or_default();
        let raw_datas = client
            .get(endpoint().rawdata(&device_id))
            .header("CK", project_id)
            .send()
            .await?
            .json::<Vec<RawData>>()
            .await?;
        let raw_data_map: HashMap<String, RawData> =
            raw_datas.into_iter().map(|d| (d.id.clone(), d)).collect();
        let sensor_data: Vec<_> = sensors
            .into_iter()
            .map(|s| SensorWithData {
                sensor: s.clone(),
                data: raw_data_map.get(&s.id).cloned(),
            })
            .collect();

        Ok(sensor_data)
    });

    use_future(move || async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            *timer.write() -= 1;
            if timer() < 0 {
                timer.set(10);
                if resource.finished() {
                    resource.restart();
                }
            }
        }
    });

    rsx! {
        div { class: "flex w-full justify-end my-4 gap-4",
            Button {
                onclick: move |_| {
                    resource.restart();
                    timer.set(10);
                },
                "Refresh: {timer()}"
            }
            Button { "Add Sensor(TODO)" }
        }
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
            if let Some(response) = &*resource.read() {
                match response {
                    Ok(sensors) => rsx! {
                        for s in sensors {
                            SensorPanel3 {
                                project,
                                endpoint,
                                device,
                                ctx,
                                sensor_data: s.clone(),
                            }
                        }
                    },
                    Err(err) => rsx! { "Failed to fetch response: {err}" },
                }
            } else {
                "Loading..."
            }
        }
    }
}

#[component]
pub fn SensorPanel3(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    ctx: Store<PageContext>,
    sensor_data: ReadSignal<SensorWithData>,
) -> Element {
    let data = use_memo(move || sensor_data().data);
    let value = use_memo(move || {
        data()
            .map(|v| {
                v.value
                    .iter()
                    .map(|e| e.clone().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    });

    let sensor = use_memo(move || sensor_data().sensor);

    let img_data: Resource<Result<_, Error>> = use_resource(move || async move {
        let sensor = sensor_data().sensor;
        let data = sensor_data().data;
        let first_value = data
            .map(|d| {
                d.value
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if sensor.kind == SensorType::Snapshot && first_value.len() > 11 {
            let client = reqwest::Client::new();
            let sensor_id = sensor.id;
            let device_id = device().id;
            let project_key = project().project_key;
            let snapshot_id = first_value[11..].to_string();
            let url = endpoint().snapshot(&device_id, &sensor_id, &snapshot_id);
            let img = client
                .get(url)
                .header("CK", project_key.as_str())
                .send()
                .await?
                .bytes()
                .await?;
            let img_b64 = String::from("data:image/jpeg;base64,") + &BASE64_STANDARD.encode(img);
            Ok(img_b64)
        } else {
            Err(anyhow!("Not a snapshot"))
        }
    });

    let sensor_id = use_memo(move || sensor().id.clone());

    let btnclick = move |_| ctx.view_sensor_attr(&sensor_id());
    let history_click = move |_| ctx.view_sensor_history(&sensor_id());

    rsx! {
        Card {
            CardHeader {
                // CardTitle displays the main heading.
                CardTitle { {sensor().name} }
                CardAction {
                    // Button { variant: ButtonVariant::Ghost, onclick: btnclick,
                    //     Icon { icon: fa_solid_icons::FaSliders }
                    // }
                    // Button { variant: ButtonVariant::Ghost, onclick: history_click,
                    //     Icon { icon: fa_solid_icons::FaClockRotateLeft }
                    // }
                    DropdownMenu {
                        DropdownMenuTrigger {
                            Icon { icon: fa_solid_icons::FaEllipsisVertical }
                        }
                        DropdownMenuContent {
                            DropdownMenuItem::<String> {
                                value: "History".to_string(),
                                index: 0usize,
                                on_select: history_click,
                                div { class: "flex gap-2",
                                    Icon { icon: fa_solid_icons::FaClockRotateLeft }
                                    "History"
                                }
                            }
                            DropdownMenuItem::<String> {
                                value: "Attributes".to_string(),
                                index: 1usize,
                                on_select: btnclick,
                                div { class: "flex gap-2",
                                    Icon { icon: fa_solid_icons::FaSliders }
                                    "Attributes"
                                }
                            }
                            DropdownMenuItem::<String> { value: "Delete".to_string(), index: 2usize,
                                div { class: "flex gap-2",
                                    Icon { icon: fa_solid_icons::FaTrash }
                                    "Delete(TODO)"
                                }
                            }
                        }
                    }
                }
            }
            // CardContent holds the main body content.
            CardContent {
                div { class: "max-h-32 h-32 flex justify-center items-center",
                    if let Some(image_data) = &*img_data.read() {
                        match image_data {
                            Ok(image_data) => rsx! {
                                img { class: "h-32 object-contain", src: image_data.as_str() }
                            },
                            Err(_) => rsx! {
                                p { class: "text-2xl font-bold truncate", "{value}" }

                            },
                        }
                    } else {
                        p { class: "text-2xl font-bold truncate", "{value}" }
                    }
                }
            }
            // CardFooter contains footer actions or information.
            CardFooter {
                div { class: "flex flex-col",
                    p { {data().map(|d| d.time).unwrap_or_default()} }
                    p { {sensor().id} }
                }
            }
        }
    }
}

#[component]
pub fn DeviceAttrPanel(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let panel = if project().is_some() && endpoint().is_some() && device().is_some() {
        let project = project().unwrap();
        let endpoint = endpoint().unwrap();
        let device = device().unwrap();

        rsx! {
            DeviceAttrPanelImpl {
                project,
                endpoint,
                device,
                ctx,
                project_meta,
            }
        }
    } else {
        rsx! {
            p { "Error" }
        }
    };

    rsx! {
        DeviceHeader {
            project,
            endpoint,
            device,
            ctx,
        }
        {panel}
    }
}

#[component]
pub fn DeviceAttrPanelImpl(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let mut attributes = use_signal(|| device().attributes.unwrap_or_default().clone());
    let is_dirty = use_memo(move || attributes() != device().attributes.unwrap_or_default());

    let mut device_info = use_signal(|| device().clone());
    let is_device_dirty = use_memo(move || device_info() != device());

    let save_attrs = move |_| async move {
        let edit_device = EditDevice {
            name: device().name.clone(),
            kind: device().kind.clone(),
            attributes: Some(attributes().clone()),
            ..Default::default()
        };
        let client = Client::new();
        let url = endpoint().device(&device().id);
        let toastapi = use_toast();

        let json_text = serde_json::to_string(&edit_device);
        tracing::debug!("{:?}", json_text);
        let result = client
            .put(url)
            .header("CK", &project().project_key)
            .json(&edit_device)
            .send()
            .await;

        match result {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                project_meta.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    let save_device_info = move |_| async move {
        let edit_device = EditDevice {
            name: device_info().name.clone(),
            kind: device().kind.clone(),
            desc: device_info().desc.clone(),
            ..Default::default()
        };
        let client = Client::new();
        let url = endpoint().device(&device().id);
        let toastapi = use_toast();

        let json_text = serde_json::to_string(&edit_device);
        tracing::debug!("{:?}", json_text);
        let result = client
            .put(url)
            .header("CK", &project().project_key)
            .json(&edit_device)
            .send()
            .await;

        match result {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                project_meta.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    rsx! {
        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Device Info" }
            Button {
                variant: if is_device_dirty() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                onclick: save_device_info,
                "Save"
            }
        }
        div { class: "grid grid-cols-[auto_auto] gap-2 w-full",
            div { "ID" }
            div { {device().id} }
            div { "Type" }
            div { {device().kind} }
            div { "Name" }
            div {
                Input {
                    class: "input w-full",
                    oninput: move |i: FormEvent| { device_info.write().name = i.value() },
                    value: device_info().name,
                }
            }
            div { "Desc" }
            div {
                Textarea {
                    oninput: move |i: FormEvent| { device_info.write().desc = Some(i.value()) },
                    value: device_info().desc,
                }
            }
        }

        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Attributes" }
            div {
                Button {
                    class: "m-4",
                    onclick: move |_| {
                        attributes
                            .write()
                            .push(Attribute {
                                key: String::new(),
                                value: String::new(),
                            });
                    },
                    "Add"
                }
                Button {
                    variant: if is_dirty() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                    onclick: save_attrs,
                    "Save"
                }
            }
        }
        div {
            for (i , attr) in attributes().iter().enumerate() {
                div { class: "flex gap-4 mb-8",
                    div { class: "flex flex-1 gap-4 flex-wrap",
                        Input {
                            class: "input flex-1",
                            placeholder: "Key",
                            onchange: move |e: FormEvent| {
                                attributes.write()[i].key = e.value();
                            },
                            value: attr.key.clone(),
                        }
                        Input {
                            class: "input flex-1",

                            placeholder: "Value",
                            onchange: move |e: FormEvent| {
                                attributes.write()[i].value = e.value();
                            },
                            value: attr.value.clone(),
                        }
                    
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            attributes.write().remove(i);
                        },
                        Icon { icon: fa_solid_icons::FaXmark }
                    }
                
                }
            }
        }

        MonitorPanel { project, endpoint, device }
    }
}

#[component]
pub fn SensorAttrPanel(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    sensor: Memo<Option<Sensor>>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let panel = if project().is_some()
        && endpoint().is_some()
        && device().is_some()
        && sensor().is_some()
    {
        let project = project().unwrap();
        let endpoint = endpoint().unwrap();
        let device = device().unwrap();
        let sensor = sensor().unwrap();

        rsx! {
            SensorAttrPanelImpl {
                project,
                endpoint,
                device,
                sensor,
                ctx,
                project_meta,
            }
        }
    } else {
        rsx! {
            p { "Error" }
        }
    };

    rsx! {
        DeviceHeader {
            project,
            endpoint,
            device,
            ctx,
        }
        SensorHeader {
            project,
            endpoint,
            device,
            sensor,
            ctx,
        }

        {panel}
    }
}

#[component]
pub fn SensorHeader(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    sensor: Memo<Option<Sensor>>,
    ctx: Store<PageContext>,
) -> Element {
    let back = move |_| ctx.back_to_sensors();

    let back_btn = rsx! {
        Button { variant: ButtonVariant::Ghost, onclick: back,
            Icon { icon: fa_solid_icons::FaArrowLeft }
        }
    };

    let sensor_card = if let Some(sensor) = sensor() {
        let desc = sensor.desc.unwrap_or_default();
        rsx! {
            Card {
                CardHeader {
                    CardTitle { {sensor.name} }
                    CardDescription { {desc} }
                    CardAction { {back_btn} }
                }
                CardContent {
                    p { {sensor.id} }
                }
            }

        }
    } else {
        rsx! {
            Card {
                CardHeader {
                    CardTitle { "Error" }
                    CardAction { {back_btn} }
                }
            }
        }
    };

    rsx! {
        {sensor_card}
    }
}

#[component]
pub fn SensorAttrPanelImpl(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    sensor: ReadSignal<Sensor>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let mut attributes = use_signal(|| sensor().attributes.unwrap_or_default().clone());
    let is_dirty = use_memo(move || attributes() != sensor().attributes.unwrap_or_default());

    let save_attrs = move |_| async move {
        let edit_sensor = EditSensor {
            name: sensor().name.clone(),
            kind: sensor().kind.clone(),
            attributes: Some(attributes().clone()),
            ..Default::default()
        };
        let client = Client::new();
        let url = endpoint().sensor(&device().id, &sensor().id);
        let toastapi = use_toast();

        let result = client
            .put(url)
            .header("CK", &project().project_key)
            .json(&edit_sensor)
            .send()
            .await;

        match result {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(10)),
                );
                project_meta.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Attributes" }
                CardAction {
                    Button {
                        class: "m-4",
                        onclick: move |_| {
                            attributes
                                .write()
                                .push(Attribute {
                                    key: String::new(),
                                    value: String::new(),
                                });
                        },
                        "Add"
                    }
                    Button {
                        variant: if is_dirty() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                        onclick: save_attrs,
                        "Save"
                    }
                
                }
            }
            CardContent {
                for (i , attr) in attributes().iter().enumerate() {
                    div { class: "flex gap-4 mb-8",
                        div { class: "flex flex-1 gap-4 flex-wrap",
                            Input {
                                class: "input flex-1",
                                placeholder: "Key",
                                onchange: move |e: FormEvent| {
                                    attributes.write()[i].key = e.value();
                                },
                                value: attr.key.clone(),
                            }
                            Input {
                                class: "input flex-1",

                                placeholder: "Value",
                                onchange: move |e: FormEvent| {
                                    attributes.write()[i].value = e.value();
                                },
                                value: attr.value.clone(),
                            }
                        
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| {
                                attributes.write().remove(i);
                            },
                            Icon { icon: fa_solid_icons::FaXmark }
                        }
                    
                    }
                }
            }
        }
    }
}

#[component]
pub fn MonitorPanel(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
) -> Element {
    let active_status = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = endpoint().active(&device().id);
        client
            .get(url)
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Option<ActiveInfo>>()
            .await
    });

    let active_setting: Resource<Result<ActiveDevice>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = endpoint().active_setting(&device().id);
        let ret = client
            .get(url)
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<ActiveDevice>()
            .await?;
        Ok(ret)
    });

    let active_notify: Resource<Result<Vec<ActiveNotify>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = endpoint().active_notify(&device().id);
        let ret = client
            .get(url)
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Vec<ActiveNotify>>()
            .await?;
        Ok(ret)
    });

    let active_rsx = if let Some(active_status) = &*active_status.read() {
        match active_status {
            Ok(active_status) => {
                if let Some(active_status) = active_status {
                    rsx! {
                        div {
                            p { "{active_status.status}" }
                            p { "{active_status.create_time}" }
                        }
                    }
                } else {
                    rsx! {
                        p { "Unset" }
                    }
                }
            }
            Err(_) => rsx! {
                p { "Load Error" }
            },
        }
    } else {
        rsx! {
            p { "Loading" }
        }
    };

    let active_setting_rsx = if let Some(setting) = &*active_setting.read() {
        match setting {
            Ok(setting) => {
                rsx! {
                    ActiveSettingSection {
                        project,
                        endpoint,
                        device,
                        setting: setting.clone(),
                        active_setting,
                    }
                }
            }
            Err(_) => rsx! {
                p { "Load Error" }
            },
        }
    } else {
        rsx! {
            p { "Loading" }
        }
    };

    let active_notify_rsx = if let Some(notifies) = &*active_notify.read() {
        match notifies {
            Ok(notifies) => rsx! {
                ActiveNotifySection {
                    project,
                    endpoint,
                    device,
                    notifies: notifies.clone(),
                    active_notify,
                }
            },
            Err(_) => rsx! {
                p { "Load Error" }
            },
        }
    } else {
        rsx! {
            p { "Loading" }
        }
    };

    rsx! {
        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Active Monitor Status" }
        }
        {active_rsx}

        {active_setting_rsx}

        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Active Notifications" }
        }
        {active_notify_rsx}

    }
}

#[component]
fn ActiveNotifyCard(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    notify: ReadSignal<ActiveNotify>,
    active_notify: Resource<Result<Vec<ActiveNotify>>>,
) -> Element {
    let mut notify_edit = use_signal(move || notify().clone());
    let is_dirty_varient = use_memo(move || {
        if notify_edit() == notify() {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Primary
        }
    });
    let mut delete_dialog_open = use_signal(|| false);

    let on_save_click = move |_| async move {
        let client = reqwest::Client::new();
        let edit_notify = notify_edit();
        let toastapi = use_toast();

        if edit_notify.setting.to.is_empty() {
            toastapi.error(
                "'To' email cannot be empty".to_string(),
                ToastOptions::new().duration(Duration::from_secs(10)),
            );
            return;
        }

        let url = endpoint().active_notify(&device().id);
        let result = client
            .post(url)
            .header("CK", project().project_key.as_str())
            .json(&edit_notify)
            .send()
            .await;

        match result {
            Ok(data) => {
                let status = data.status();
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());

                if status.is_success() {
                    toastapi.success(
                        "Updated".to_string(),
                        ToastOptions::new()
                            .description(text)
                            .duration(Duration::from_secs(5)),
                    );
                } else {
                    toastapi.error(
                        "Update Failed".to_string(),
                        ToastOptions::new()
                            .description(format!("{status} {text}"))
                            .duration(Duration::from_secs(10)),
                    );
                    return;
                }
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
                return;
            }
        }

        let delete_url = endpoint().active_notify_delete(&device().id, notify_edit().id);
        let ret1 = client
            .delete(delete_url)
            .header("CK", project().project_key.as_str())
            .send()
            .await;

        match ret1 {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());

                toastapi.success(
                    "Delete".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(10)),
                );
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
        active_notify.restart();
    };

    let on_delete_click = move |_| async move {
        delete_dialog_open.set(false);

        let client = reqwest::Client::new();
        let toastapi = use_toast();
        let delete_url = endpoint().active_notify_delete(&device().id, notify().id);
        let ret1 = client
            .delete(delete_url)
            .header("CK", project().project_key.as_str())
            .send()
            .await;

        match ret1 {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());

                toastapi.success(
                    "Delete".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                active_notify.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    let delete_dialog = rsx! {
        DialogRoot {
            open: delete_dialog_open(),
            on_open_change: move |v| delete_dialog_open.set(v),
            DialogContent {
                DialogTitle { "Delete Confirm" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        "Delete id {notify().id}"
                        div { class: "flex justify-end gap-4",
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_dialog_open.set(false),
                                "NO"
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: on_delete_click,
                                "Yes"
                            }
                        }
                    
                    }
                }
            }
        }
    };

    rsx! {
        {delete_dialog}
        div { class: "border rounded-lg p-4 relative",
            div { class: "absolute top-2 right-2 flex space-x-2",
                Button { variant: is_dirty_varient(), onclick: on_save_click, "Save" }
                Button {
                    variant: ButtonVariant::Destructive,
                    onclick: move |_| delete_dialog_open.set(true),
                    "Delete"
                }
            }
            div { class: "grid grid-cols-[auto_auto] gap-2",
                div { class: "font-semibold", "ID:" }
                div { {notify().id.to_string()} }
                div { class: "font-semibold", "Name:" }
                div {
                    Input {
                        value: notify_edit().name,
                        oninput: move |e: FormEvent| notify_edit.write().name = e.value(),
                    }
                }
                div { class: "font-semibold", "Type:" }
                div {
                    Input {
                        value: notify_edit().kind,
                        oninput: move |e: FormEvent| notify_edit.write().kind = e.value(),
                    }
                }
                div { class: "font-semibold", "Enabled:" }
                div { class: "flex items-center gap-4",
                    Switch {
                        checked: notify_edit().enable,
                        on_checked_change: move |v| notify_edit.write().enable = v,
                        SwitchThumb {}
                    }
                    "{notify_edit().enable}"
                }
                div { class: "font-semibold", "To:" }
                div {
                    Textarea {
                        value: notify_edit().setting.to,
                        oninput: move |e: FormEvent| notify_edit.write().setting.to = e.value(),
                    }
                }
                div { class: "font-semibold", "Message:" }
                div {
                    Textarea {
                        value: notify_edit().setting.message,
                        oninput: move |e: FormEvent| notify_edit.write().setting.message = Some(e.value()),
                    }
                }
                div { class: "font-semibold", "Created:" }
                div { {notify().create_time} }
            }
        }
    }
}

#[component]
fn ActiveNotifySection(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    notifies: Vec<ActiveNotify>,
    active_notify: Resource<Result<Vec<ActiveNotify>>>,
) -> Element {
    let on_add_button = move |_| async move {
        let toastapi = use_toast();
        let client = reqwest::Client::new();
        let new_notify = ActiveNotify {
            id: 1,
            device_id: device().id,
            enable: true,
            name: "notify".to_string(),
            kind: "MAIL".to_string(),
            setting: ActiveNotifySetting {
                to: "somebody@some.example".to_string(),
                message: Some(String::new()),
            },
            create_time: String::new(),
        };

        let url = endpoint().active_notify(&device().id);
        let result = client
            .post(url)
            .header("CK", project().project_key.as_str())
            .json(&new_notify)
            .send()
            .await;

        match result {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                active_notify.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    if notifies.is_empty() {
        rsx! {
            div {
                p { "No notifications configured" }
                Button { onclick: on_add_button, "Add" }
            }
        }
    } else {
        rsx! {
            div {
                div { class: "flex justify-end mb-2",
                    Button { onclick: on_add_button, "Add" }
                }
                div { class: "grid grid-cols-1 gap-4",
                    for notify in notifies {
                        ActiveNotifyCard {
                            key: "{notify.id}",
                            project,
                            endpoint,
                            device,
                            notify: notify.clone(),
                            active_notify,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ActiveSettingSection(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    setting: ReadSignal<ActiveDevice>,
    active_setting: Resource<Result<ActiveDevice, Error>>,
) -> Element {
    let mut setting_clone = use_signal(|| setting().clone());
    use_effect(move || {
        tracing::info!("Copy original value");
        setting_clone.set(setting());
    });
    let is_dirty = use_memo(move || setting() != setting_clone());
    let is_dirty_varient = use_memo(move || {
        if is_dirty() {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Secondary
        }
    });

    let save_active_setting = move |_| async move {
        let toastapi = use_toast();
        let client = reqwest::Client::new();
        let url = endpoint().active_setting(&device().id);
        let edit_active = ActiveDevice {
            device_id: device().id.clone(),
            enable: setting_clone().enable,
            period: setting_clone().period.clone(),
            min_uploads: setting_clone().min_uploads.clone(),
            max_uploads: setting_clone().max_uploads.clone(),
            sensor: setting_clone().sensor.clone(),
            create_time: setting().create_time,
        };
        let result = client
            .post(url)
            .header("CK", project().project_key.as_str())
            .json(&edit_active)
            .send()
            .await;

        match result {
            Ok(data) => {
                let text = data
                    .text()
                    .await
                    .unwrap_or_else(|_| "Error parse String".to_string());
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                active_setting.restart();
            }
            Err(e) => {
                toastapi.error(
                    "Update Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };
    let sensors = device().sensors.clone().unwrap_or_default();
    let sensor_select = sensors.iter().enumerate().map(|(i, s)| {
        rsx! {
            SelectOption::<Option<String>> { index: i + 1, value: "{s.id}", text_value: "{s.id}",
                "{s.id}"
                SelectItemIndicator {}
            }
        }
    });

    rsx! {
        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Active Setting" }
            Button { variant: is_dirty_varient(), onclick: save_active_setting, "Save" }
        }

        div { class: "grid grid-cols-[auto_auto] gap-2",
            div { "Device ID:" }
            div { {setting().device_id.clone()} }
            div { "Enabled:" }
            div { class: "flex items-center gap-4",
                Switch {
                    checked: setting_clone().enable,
                    on_checked_change: move |c| setting_clone.write().enable = c,
                    SwitchThumb {}
                }
                "{setting_clone().enable}"
            }
            div { "Period:" }
            div {
                Input {
                    value: setting_clone().period,
                    onchange: move |e: FormEvent| setting_clone.write().period = e.value(),
                }
            }
            div { "Min Uploads:" }
            div {
                Input {
                    value: setting_clone().min_uploads.map(|v| v.to_string()),
                    onchange: move |e: FormEvent| {
                        if e.value().is_empty() {
                            setting_clone.write().min_uploads = None;
                        } else {
                            let value = e.value()
                            .parse::<i32>();
                            if let Ok(value) = value {
                                setting_clone.write().min_uploads = Some(value);
                            } else {
                                setting_clone.write();
                            }
                        }
                    },
                }
            }
            div { "Max Uploads:" }
            div {
                Input {
                    value: setting_clone().max_uploads.map(|v| v.to_string()),
                    onchange: move |e: FormEvent| {
                        if e.value().is_empty() {
                            setting_clone.write().max_uploads = None;
                        } else {
                            let value = e.value()
                            .parse::<i32>();
                            if let Ok(value) = value {
                                setting_clone.write().max_uploads = Some(value);
                            } else {
                                setting_clone.write();
                            }
                        }
                    },
                }
            }
            div { "Sensor" }
            div {
                Select::<Option<String>> {
                    value: Some(setting_clone().sensor),
                    on_value_change: move |e: Option<Option<String>>| {
                        setting_clone.write().sensor = e.unwrap_or_default()
                    },
                    SelectTrigger { SelectValue {} }
                    SelectList {
                        SelectGroup {
                            SelectOption::<Option<String>> {
                                index: 0usize,
                                value: None,
                                text_value: "(All Sensor)",
                                "(All Sensor)"
                                SelectItemIndicator {}
                            }
                            {sensor_select}
                        }
                    }
                }
            }
            div { "Created:" }
            div { "{setting().create_time.unwrap_or_default()}" }
        }
    }
}

#[component]
pub fn SensorHistoryPanel(
    project: Memo<Option<Project>>,
    endpoint: Memo<Option<Endpoint>>,
    device: Memo<Option<Device>>,
    sensor: Memo<Option<Sensor>>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let panel = if project().is_some()
        && endpoint().is_some()
        && device().is_some()
        && sensor().is_some()
    {
        let project = project().unwrap();
        let endpoint = endpoint().unwrap();
        let device = device().unwrap();
        let sensor = sensor().unwrap();

        rsx! {
            SensorHistoryPanelImpl {
                project,
                endpoint,
                device,
                sensor,
                ctx,
                project_meta,
            }
        }
    } else {
        rsx! {
            p { "Error" }
        }
    };

    rsx! {
        SensorHeader {
            project,
            endpoint,
            device,
            sensor,
            ctx,
        }

        {panel}
    }
}

#[component]
pub fn SensorHistoryPanelImpl(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    sensor: ReadSignal<Sensor>,
    ctx: Store<PageContext>,
    project_meta: Resource<Result<Vec<Device>>>,
) -> Element {
    let today = use_signal(|| {
        OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .date()
    });

    let gmt8_format =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let offsets = vec![offset!(+8), offset!(+0)];

    let mut use_utc = use_signal(|| false);
    let use_offset = use_memo(move || if use_utc() { offsets[1] } else { offsets[0] });
    let mut use_asc = use_signal(|| false);

    let format = format_description!("[hour]:[minute]");

    let mut selected_datetime = use_signal(|| {
        OffsetDateTime::new_in_offset(
            today().saturating_add(time::Duration::days(1)),
            time::Time::MIDNIGHT,
            offset!(+0),
        )
    });

    let selected_date = use_memo(move || selected_datetime().date());
    let selected_time = use_memo(move || {
        selected_datetime()
            .time()
            .format(format)
            .unwrap_or_default()
    });

    let raw_datas: Resource<Result<Vec<GetRawData>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = endpoint().sensor_rawdata(&device().id, &sensor().id);
        let start;
        let end;
        if use_asc() {
            start = selected_datetime().replace_offset(use_offset());
            end = datetime!(2099-01-01 0:00 UTC);
        } else {
            start = OffsetDateTime::UNIX_EPOCH;
            end = selected_datetime().replace_offset(use_offset());
        }
        let start_str = start.to_utc().format(&Iso8601::DATE_TIME_OFFSET)?;
        let end_str = end.to_utc().format(&Iso8601::DATE_TIME_OFFSET)?;
        let asc_or_desc = if use_asc() { "ASC" } else { "DESC" };
        let mut url = Url::from_str(&url)?;
        url.query_pairs_mut()
            .append_pair("start", &start_str)
            .append_pair("end", &end_str)
            .append_pair("order", asc_or_desc);
        // if !use_utc() {
        //     url.query_pairs_mut().append_pair("utcOffset", "8");
        // }
        let ret = client
            .get(url)
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Vec<GetRawData>>()
            .await?;
        let ret2 = if use_utc() {
            ret
        } else {
            ret.iter()
                .map(|d| {
                    let t = OffsetDateTime::parse(&d.time, &Iso8601::DATE_TIME_OFFSET)
                        .map(|i| i.to_offset(offset!(+8)).format(gmt8_format))
                        .unwrap_or_else(move |_| Ok(d.time.clone()))
                        .unwrap_or_else(move |_| d.time.clone());
                    GetRawData {
                        time: t,
                        ..d.clone()
                    }
                })
                .collect()
        };

        Ok(ret2)
    });

    let selected_index = use_signal(|| None::<usize>);

    let imgdata_res: Resource<Result<Option<String>>> = use_resource(move || async move {
        tracing::info!("imgdata_res");
        if sensor().kind != SensorType::Snapshot {
            return Ok(None::<String>);
        }
        let mut snapshot_url: Option<String> = None;
        if let Some(Ok(data)) = &*raw_datas.read() {
            if let Some(selected_index) = selected_index() {
                if selected_index < data.len() {
                    let raw_data = &data[selected_index];
                    if let Some(Some(first)) = raw_data.value.first() {
                        if first.len() > 11 {
                            snapshot_url = Some(first.clone());
                        }
                    }
                }
            }
        }
        if let Some(snapshot_url) = snapshot_url {
            let client = reqwest::Client::new();
            let sensor_id = sensor().id;
            let device_id = device().id;
            let project_key = project().project_key;
            let snapshot_id = snapshot_url[11..].to_string();
            let url = endpoint().snapshot(&device_id, &sensor_id, &snapshot_id);
            let img = client
                .get(url)
                .header("CK", project_key.as_str())
                .send()
                .await?
                .bytes()
                .await?;
            let img_b64 = String::from("data:image/jpeg;base64,") + &BASE64_STANDARD.encode(img);
            return Ok(Some(img_b64));
        }
        Ok(None)
    });
    let imgdata = use_memo(move || {
        tracing::info!("imgdata");
        if let Some(Ok(Some(data))) = &*imgdata_res.read() {
            return Some(data.clone());
        }
        None
    });

    let add_one_day = move |_| {
        selected_datetime.set(selected_datetime().saturating_add(time::Duration::days(1)));
    };

    let minus_one_day = move |_| {
        selected_datetime.set(selected_datetime().saturating_sub(time::Duration::days(1)));
    };

    let add_one_hour = move |_| {
        selected_datetime.set(selected_datetime().saturating_add(time::Duration::hours(1)));
    };

    let minus_one_hour = move |_| {
        selected_datetime.set(selected_datetime().saturating_sub(time::Duration::hours(1)));
    };

    rsx! {
        h2 { class: "text-xl font-bold mb-4", "Raw Data Records" }

        div { class: "flex justify-center gap-4 flex-wrap items-center",
            div { class: "flex justify-center gap-4",
                p { "GMT+8" }
                Switch {
                    checked: use_utc(),
                    on_checked_change: move |b| use_utc.set(b),
                    SwitchThumb {}
                }
                p { "UTC" }
            }

            div { class: "flex justify-center gap-4",
                p { "DESC" }
                Switch {
                    checked: use_asc(),
                    on_checked_change: move |b| use_asc.set(b),
                    SwitchThumb {}
                }
                p { "ASC" }
            }

            div { class: "flex justify-center gap-4",
                Button { onclick: minus_one_day,
                    Icon { icon: fa_solid_icons::FaChevronLeft }
                }
                Input {
                    r#type: "date",
                    oninput: move |e: FormEvent| {
                        let d = Date::parse(e.value().as_str(), &Iso8601::DATE);
                        if let Ok(d) = d {
                            selected_datetime.set(selected_datetime().replace_date(d));
                        } else {
                            selected_datetime.set(selected_datetime().replace_date(today()));
                        }
                    },
                    value: "{selected_date()}",
                }
                Button { onclick: add_one_day,
                    Icon { icon: fa_solid_icons::FaChevronRight }
                }
            }
            div { class: "flex justify-center gap-4",
                Button { onclick: minus_one_hour,
                    Icon { icon: fa_solid_icons::FaChevronLeft }
                }
                Input {
                    r#type: "time",
                    oninput: move |e: FormEvent| {
                        let t = time::Time::parse(e.value().as_str(), &Iso8601::TIME);
                        if let Ok(t) = t {
                            selected_datetime.set(selected_datetime().replace_time(t));
                        }
                    },

                    value: "{selected_time()}",
                }
                Button { onclick: add_one_hour,
                    Icon { icon: fa_solid_icons::FaChevronRight }
                }
            }
        }

        // 顯示raw_datas的資料
        if let Some(response) = &*raw_datas.read() {
            match response {
                Ok(datas) => rsx! {
                    if datas.is_empty() {
                        div { class: "text-center py-8 text-gray-500", "No data available for this date" }
                    } else {
                        SensorRawDataTable { datas: datas.clone(), selected_index, imgdata }
                    }
                },
                Err(err) => rsx! {
                    div { class: "text-red-500 mt-4", "Error loading data: {err}" }
                },
            }
        } else {
            div { class: "text-gray-500 mt-4", "Loading data..." }
        }
    }
}

#[component]
pub fn SensorRawDataTable(
    datas: ReadSignal<Vec<GetRawData>>,
    selected_index: Signal<Option<usize>>,
    imgdata: ReadSignal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "mt-8 mb-8",
            div {
                class: "grid grid-cols-1  gap-4",
                class: if imgdata().is_some() { "md:grid-cols-2" },
                div { class: "flex flex-col overflow-auto max-h-[60lvh]",
                    table { class: "border-collapse border border-(--primary-color-7)",
                        thead {
                            tr { class: "bg-(--primary-color-5)",
                                th { class: "border border-(--primary-color-7)  p-2 text-left text-(--secondary-color-1) ",
                                    "Time"
                                }
                                th { class: "border border-(--primary-color-7) p-2 text-left text-(--secondary-color-1) ",
                                    "Value"
                                }
                            }
                        }
                        tbody {
                            for (index , data) in datas().iter().enumerate() {
                                tr {
                                    key: "{data.time.clone()}",
                                    class: "hover:bg-(--primary-color-5)  cursor-pointer text-(--secondary-color-1)",
                                    class: if selected_index() == Some(index) { "bg-blue-100 dark:bg-blue-900" },
                                    onclick: move |_| {
                                        selected_index.set(Some(index));
                                    },
                                    td { class: "border border-(--primary-color-7) p-2 font-mono text-sm",
                                        "{data.time.clone()}"
                                    }
                                    td { class: "border border-(--primary-color-7) p-2 font-mono text-sm",
                                        "{data.all_value()}"
                                    }
                                }
                            }
                        }
                    }
                }
                if imgdata().is_some() {
                    div { class: "grid justify-center items-center",
                        img { class: "object-contain", src: imgdata() }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SensorRawDataGrid(datas: Vec<GetRawData>) -> Element {
    rsx! {
        div { class: "mt-8",
            h2 { class: "text-xl font-bold mb-4", "Raw Data Records" }
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 p-4",
                for data in &datas {
                    div { class: "border rounded-lg p-4 bg-white shadow-sm hover:shadow-md transition-shadow",
                        div { class: "mb-3",
                            div { class: "text-xs font-semibold text-gray-500 uppercase mb-1",
                                "Time"
                            }
                            p { class: "text-sm font-mono text-gray-900 wrap-break-word",
                                "{data.time.clone()}"
                            }
                        }
                        div {
                            div { class: "text-xs font-semibold text-gray-500 uppercase mb-1",
                                "Value"
                            }
                            p { class: "text-sm font-mono text-gray-900 wrap-break-word",
                                "{data.all_value()}"
                            }
                        }
                    }
                }
            }
        }
    }
}
