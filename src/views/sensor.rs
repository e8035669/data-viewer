use std::collections::HashMap;
use std::time::Duration;

use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use crate::components::input::Input;
use crate::views::endpoints::{Endpoint, EndpointTrait, Endpoints};
use crate::views::projects::{Project, Projects};
use anyhow::{anyhow, Error, Result};
use base64::prelude::*;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons;
use dioxus_free_icons::Icon;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum SensorType {
    #[default]
    Gauge,
    Text,
    Switch,
    Snapshot,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Debug, Default)]

pub struct Sensor {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    #[serde(rename = "type")]
    pub kind: SensorType,
    pub uri: Option<String>,
    pub formula: Option<String>,
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Debug, Default)]

pub struct EditSensor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(rename = "type")]
    pub kind: SensorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Debug, Store, Default)]

pub struct Device {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub uri: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub attributes: Option<Vec<Attribute>>,
    pub sensors: Option<Vec<Sensor>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Debug, Store, Default)]

pub struct EditDevice {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Debug)]
pub struct RawData {
    pub id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub value: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SensorWithData {
    pub sensor: Sensor,
    pub data: Option<RawData>,
}

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
}

#[component]
pub fn DevicePage3(project_name: ReadSignal<String>) -> Element {
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
        let data = client
            .get(url)
            .header("CK", project_id.as_str())
            .send()
            .await?
            .json::<Vec<Device>>()
            .await?;
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
pub fn DevicesPanels3(devices: Vec<Device>, ctx: Store<PageContext>) -> Element {
    rsx! {
        for d in devices {
            DevicePanel3 { device: d.clone(), ctx }
        }
    }
}

#[component]
pub fn DevicePanel3(device: Device, ctx: Store<PageContext>) -> Element {
    let device_clone = device.clone();
    let device_clone2 = device.clone();
    let desc = device.desc.unwrap_or_default();
    let view_sensor = move |_| ctx.view_sensors(&device_clone.id);
    let view_device_attr = move |_| ctx.view_device_attr(&device_clone2.id);
    rsx! {
        div { class: " mb-2",
            Card {
                CardHeader {
                    CardTitle { {device.name} }
                    CardDescription { {desc} }
                    CardAction {
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: view_device_attr,
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
    let resource: Resource<Result<_, Error>> = use_resource(move || async move {
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

    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 p-4",
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
    let data = sensor_data().data;
    let value = data.map(|d| d.value.join(" ")).unwrap_or_default();
    let value = use_signal(|| value);

    let sensor = sensor_data().sensor;

    let img_data: Resource<Result<_, Error>> = use_resource(move || async move {
        let sensor = sensor_data().sensor;
        if sensor.kind == SensorType::Snapshot && value().len() > 11 {
            let client = reqwest::Client::new();
            let sensor_id = sensor.id;
            let device_id = device().id;
            let project_key = project().project_key;
            let snapshot_id = value()[11..].to_string();
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

    let sensor_id = sensor.id.clone();
    let btnclick = move |_| ctx.view_sensor_attr(&sensor_id);

    rsx! {
        Card {
            CardHeader {
                // CardTitle displays the main heading.
                CardTitle { "{sensor.name}" }
                CardAction {
                    Button { variant: ButtonVariant::Ghost, onclick: btnclick,
                        Icon { icon: fa_solid_icons::FaSliders }
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
                p { "{sensor.id}" }
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
