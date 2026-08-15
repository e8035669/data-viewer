use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::{
    api::{ApiHelper, SensorRawDataQuery},
    components::{
        button::{Button, ButtonVariant},
        card::{Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle},
        dialog::{Dialog, DialogDescription, DialogTitle},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem},
        label::Label,
        radio_group::{RadioGroup, RadioItem},
        select::{Select, SelectGroup, SelectOption},
        switch::Switch,
        textarea::Textarea,
    },
    models::{
        ActiveDevice, ActiveInfo, ActiveNotify, ActiveNotifySetting, Attribute, Device, EditDevice,
        EditSensor, Endpoint, Endpoints, GetRawData, Project, Projects, RawData, Sensor,
        SensorStoreExt, SensorType, SensorWithData,
    },
    ui::{
        breadcrumb::{Breadcrumb, BreadcrumbItem},
        custom::{DxDropdownMenuContent, DxDropdownMenuTrigger, DxInput},
        page_header::PageHeader,
    },
    views::global::HeaderContext,
    Route,
};
use anyhow::{anyhow, Error, Result};
use async_std::task::sleep;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;
use strum::IntoEnumIterator;
use time::format_description::well_known::Iso8601;
use time::macros::{datetime, format_description, offset};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectContext {
    pub(crate) project: ReadSignal<Project>,
    pub(crate) endpoint: ReadSignal<Endpoint>,
    pub(crate) project_meta: ReadSignal<Vec<Device>>,
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
                        .description(format!("ID: {ret}"))
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
        Dialog {
            open: new_device_open(),
            on_open_change: move |v| new_device_open.set(v),
            DialogTitle { "New Device" }
            div { class: "grid grid-cols-1 gap-4",
                Label { html_for: "add_device_name", "Name" }
                DxInput {
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
        Dialog {
            open: *delete_ctx.is_open().read(),
            on_open_change: move |v| delete_ctx.is_open().set(v),
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
    };

    let mut import_dialog_open = use_signal(|| false);

    let on_export_settings = move |_| {
        let data = project_meta();
        let project_key = project().project_key.clone();
        spawn(async move {
            let Ok(json) = serde_json::to_string_pretty(&data) else {
                return;
            };
            let filename = format!("{project_key}-metadata.json");
            let eval = document::eval(
                r#"
                let data = await dioxus.recv();
                let filename = await dioxus.recv();
                const blob = new Blob([data], { type: "application/json" });
                const url = URL.createObjectURL(blob);
                const link = document.createElement('a');
                link.href = url;
                link.download = filename;
                document.body.appendChild(link);
                link.click();
                document.body.removeChild(link);
                URL.revokeObjectURL(url);
                "#,
            );
            let _ = eval.send(json);
            let _ = eval.send(filename);
        });
    };

    rsx! {
        Breadcrumb { items: vec![BreadcrumbItem::link("Projects", Route::ProjectsView {})] }
        PageHeader { title: "Devices",
            Button {
                variant: ButtonVariant::Ghost,
                onclick: move |_| import_dialog_open.set(true),
                Icon { icon: fa_solid_icons::FaFileImport }
                "匯入設定"
            }
            Button { variant: ButtonVariant::Ghost, onclick: on_export_settings,
                Icon { icon: fa_solid_icons::FaFileExport }
                "匯出設定"
            }
            Link {
                to: Route::ExportDataPage {
                    project_name: project_name(),
                },
                class: "button flex items-center gap-2",
                "data-style": "ghost",
                Icon { icon: fa_solid_icons::FaFileCsv }
                "匯出資料"
            }
            Link {
                to: Route::ExportSnapshotsPage {
                    project_name: project_name(),
                },
                class: "button flex items-center gap-2",
                "data-style": "ghost",
                Icon { icon: fa_solid_icons::FaImages }
                "匯出快照"
            }
            Link {
                to: Route::ProjectAttributeBatch {
                    project_name: project_name(),
                },
                class: "button flex items-center gap-2",
                "data-style": "ghost",
                Icon { icon: fa_solid_icons::FaPenToSquare }
                "批次修改屬性"
            }
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
        ImportSettingsDialog {
            open: import_dialog_open,
            project,
            endpoint,
            project_meta,
            project_resource,
        }
        div { class: "grid sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 items-start",
            if project_meta().is_empty() {
                p { "No device, Add new one" }
            }
            for d in project_meta() {
                DeviceCard {
                    key: "{d.id}",
                    project_name,
                    device: d.clone(),
                    delete_ctx,
                }
            }
        }
        div { class: "h-48" }
    }
}

#[component]
pub fn DeviceCard(
    project_name: ReadSignal<String>,
    device: ReadSignal<Device>,
    delete_ctx: Store<DeleteCtx>,
) -> Element {
    let ProjectContext {
        project, endpoint, ..
    } = use_context();
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();

    let desc = device().desc.unwrap_or_default();
    let nav2 = navigator();
    let view_device_attr = move |_| {
        nav2.push(Route::DeviceAttr {
            project_name: project_name(),
            device_id: device().id,
        });
    };
    let on_delete = move |_| {
        delete_ctx.prompt_delete(&device().id);
    };
    let on_duplicate = move |_| async move {
        let toast_api = use_toast();
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let dev = device();
        let ret = ApiHelper::duplicate_device(&client, &ep, &pk, &dev).await;
        match ret {
            Ok(new_id) => {
                toast_api.success(
                    "複製成功".to_string(),
                    ToastOptions::new()
                        .description(format!("新裝置 ID: {new_id}"))
                        .duration(Duration::from_secs(10)),
                );
                project_resource.restart();
            }
            Err(e) => {
                toast_api.error(
                    "複製失敗".to_string(),
                    ToastOptions::new()
                        .description(format!("{e:?}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };
    rsx! {
        Card {
            CardHeader {
                CardTitle { {device().name} }
                CardDescription { {desc} }
                CardAction {
                    DropdownMenu {
                        DxDropdownMenuTrigger {
                            Icon { icon: fa_solid_icons::FaEllipsisVertical }
                        }
                        DxDropdownMenuContent {
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
                                value: "Duplicate".to_string(),
                                on_select: on_duplicate,
                                div { class: "flex gap-2",
                                    Icon { icon: fa_solid_icons::FaCopy }
                                    "複製"
                                }
                            }
                            DropdownMenuItem::<String> {
                                index: 2usize,
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
            CardFooter {
                p { {device().id} }
                Link {
                    to: Route::DeviceSensors {
                        project_name: project_name(),
                        device_id: device().id,
                    },
                    class: "ml-auto button flex items-center gap-2",
                    "data-style": "ghost",
                    "Open"
                    Icon { icon: fa_solid_icons::FaArrowRight }
                }
            }
        }
    }
}

// ─── Import Settings (upload file → diff against current meta → apply) ──────

#[derive(Clone, Copy, PartialEq, Default)]
enum ImportStep {
    #[default]
    Upload,
    Preview,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ChangeKind {
    Create,
    Update,
    Delete,
}

impl ChangeKind {
    fn label_and_class(&self) -> (&'static str, &'static str) {
        match self {
            ChangeKind::Create => (
                "新增",
                "text-green-600 dark:text-green-400 border-green-600 dark:border-green-400",
            ),
            ChangeKind::Update => (
                "更新",
                "text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-400",
            ),
            ChangeKind::Delete => (
                "刪除",
                "text-red-600 dark:text-red-400 border-red-600 dark:border-red-400",
            ),
        }
    }
}

#[derive(Clone, PartialEq)]
struct SensorDiff {
    kind: ChangeKind,
    id: String,
    before: Option<Sensor>,
    after: Option<Sensor>,
    /// Set when this entry can't be safely applied (blank id, or an id duplicated
    /// elsewhere in the same device's uploaded sensor list).
    invalid_reason: Option<String>,
}

#[derive(Clone, PartialEq)]
struct DeviceDiff {
    kind: ChangeKind,
    id: String,
    before: Option<Device>,
    after: Option<Device>,
    sensor_diffs: Vec<SensorDiff>,
}

fn device_without_sensors(d: &Device) -> Device {
    Device {
        sensors: None,
        ..d.clone()
    }
}

fn device_changed_fields(before: &Device, after: &Device) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if before.name != after.name {
        fields.push("名稱");
    }
    if before.desc != after.desc {
        fields.push("描述");
    }
    if before.kind != after.kind {
        fields.push("類型");
    }
    if before.uri != after.uri {
        fields.push("URI");
    }
    if before.lat != after.lat || before.lon != after.lon {
        fields.push("經緯度");
    }
    if before.attributes != after.attributes {
        fields.push("屬性");
    }
    fields
}

fn sensor_changed_fields(before: &Sensor, after: &Sensor) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if before.name != after.name {
        fields.push("名稱");
    }
    if before.desc != after.desc {
        fields.push("描述");
    }
    if before.kind != after.kind {
        fields.push("類型");
    }
    if before.uri != after.uri {
        fields.push("URI");
    }
    if before.formula != after.formula {
        fields.push("公式");
    }
    if before.attributes != after.attributes {
        fields.push("屬性");
    }
    fields
}

fn diff_sensors(current: &[Sensor], uploaded: &[Sensor]) -> Vec<SensorDiff> {
    let current_by_id: HashMap<&str, &Sensor> = current
        .iter()
        .filter(|s| !s.id.is_empty())
        .map(|s| (s.id.as_str(), s))
        .collect();
    // Count non-blank ids so duplicates within this device's uploaded sensors can be flagged;
    // applying them in sequence would otherwise let the later one silently overwrite the earlier.
    let mut id_counts: HashMap<&str, usize> = HashMap::new();
    for s in uploaded {
        if !s.id.is_empty() {
            *id_counts.entry(s.id.as_str()).or_insert(0) += 1;
        }
    }
    let mut uploaded_ids: HashSet<&str> = HashSet::new();
    let mut diffs = Vec::new();

    for s in uploaded {
        if s.id.is_empty() {
            diffs.push(SensorDiff {
                kind: ChangeKind::Create,
                id: String::new(),
                before: None,
                after: Some(s.clone()),
                invalid_reason: Some("感測器 ID 不可為空白".to_string()),
            });
            continue;
        }
        uploaded_ids.insert(s.id.as_str());
        let duplicated = id_counts.get(s.id.as_str()).copied().unwrap_or(0) > 1;
        let invalid_reason = duplicated.then(|| "同一裝置內感測器 ID 重複".to_string());
        match current_by_id.get(s.id.as_str()) {
            None => diffs.push(SensorDiff {
                kind: ChangeKind::Create,
                id: s.id.clone(),
                before: None,
                after: Some(s.clone()),
                invalid_reason,
            }),
            Some(cur) => {
                if duplicated || *cur != s {
                    diffs.push(SensorDiff {
                        kind: ChangeKind::Update,
                        id: s.id.clone(),
                        before: Some((*cur).clone()),
                        after: Some(s.clone()),
                        invalid_reason,
                    });
                }
            }
        }
    }

    for s in current {
        if !s.id.is_empty() && !uploaded_ids.contains(s.id.as_str()) {
            diffs.push(SensorDiff {
                kind: ChangeKind::Delete,
                id: s.id.clone(),
                before: Some(s.clone()),
                after: None,
                invalid_reason: None,
            });
        }
    }

    diffs
}

/// Diffs `uploaded` metadata against `current` metadata. Devices/sensors are matched by id:
/// a blank or unknown id means create, a known id means check-for-update, and a current id
/// missing from `uploaded` means delete.
fn compute_import_plan(current: &[Device], uploaded: &[Device]) -> Vec<DeviceDiff> {
    let current_by_id: HashMap<&str, &Device> = current
        .iter()
        .filter(|d| !d.id.is_empty())
        .map(|d| (d.id.as_str(), d))
        .collect();
    let mut uploaded_ids: HashSet<&str> = HashSet::new();
    let mut plan = Vec::new();

    for d in uploaded {
        let uploaded_sensors = d.sensors.clone().unwrap_or_default();

        if d.id.is_empty() || !current_by_id.contains_key(d.id.as_str()) {
            if !d.id.is_empty() {
                uploaded_ids.insert(d.id.as_str());
            }
            plan.push(DeviceDiff {
                kind: ChangeKind::Create,
                id: String::new(),
                before: None,
                after: Some(d.clone()),
                sensor_diffs: diff_sensors(&[], &uploaded_sensors),
            });
            continue;
        }

        uploaded_ids.insert(d.id.as_str());
        let cur = current_by_id[d.id.as_str()];
        let cur_sensors = cur.sensors.clone().unwrap_or_default();
        let sensor_diffs = diff_sensors(&cur_sensors, &uploaded_sensors);
        let device_changed = device_without_sensors(cur) != device_without_sensors(d);

        if device_changed || !sensor_diffs.is_empty() {
            plan.push(DeviceDiff {
                kind: ChangeKind::Update,
                id: d.id.clone(),
                before: Some(cur.clone()),
                after: Some(d.clone()),
                sensor_diffs,
            });
        }
    }

    for d in current {
        if !d.id.is_empty() && !uploaded_ids.contains(d.id.as_str()) {
            plan.push(DeviceDiff {
                kind: ChangeKind::Delete,
                id: d.id.clone(),
                before: Some(d.clone()),
                after: None,
                sensor_diffs: Vec::new(),
            });
        }
    }

    plan
}

/// Applies a computed import plan by calling the single-purpose `ApiHelper` endpoints in order.
/// Returns a list of human-readable error messages for any operation that failed; the rest
/// still get applied so a partial failure doesn't block unrelated changes.
async fn execute_import_plan(
    client: &Client,
    endpoint: &Endpoint,
    project_key: &str,
    plan: &[DeviceDiff],
) -> Vec<String> {
    let mut errors = Vec::new();

    for d in plan {
        match d.kind {
            ChangeKind::Create => {
                let Some(after) = &d.after else { continue };
                let edit_device = EditDevice {
                    name: after.name.clone(),
                    desc: after.desc.clone(),
                    kind: after.kind.clone(),
                    uri: after.uri.clone(),
                    lat: after.lat,
                    lon: after.lon,
                    attributes: after.attributes.clone(),
                };
                match ApiHelper::create_device(client, endpoint, project_key, &edit_device).await {
                    Ok(new_id) => {
                        for sd in &d.sensor_diffs {
                            if let Some(reason) = &sd.invalid_reason {
                                errors.push(format!(
                                    "新增裝置「{}」的感測器已略過（{reason}）",
                                    after.name
                                ));
                                continue;
                            }
                            if let Some(s) = &sd.after {
                                if let Err(e) = ApiHelper::create_sensor(
                                    client,
                                    endpoint,
                                    project_key,
                                    &new_id,
                                    s,
                                )
                                .await
                                {
                                    errors.push(format!(
                                        "新增裝置「{}」的感測器「{}」失敗: {e}",
                                        after.name, s.id
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => errors.push(format!("新增裝置「{}」失敗: {e}", after.name)),
                }
            }
            ChangeKind::Update => {
                let Some(after) = &d.after else { continue };
                let device_id = d.id.clone();

                let device_changed = d
                    .before
                    .as_ref()
                    .map(|b| device_without_sensors(b) != device_without_sensors(after))
                    .unwrap_or(true);
                if device_changed {
                    let edit_device = EditDevice {
                        name: after.name.clone(),
                        desc: after.desc.clone(),
                        kind: after.kind.clone(),
                        uri: after.uri.clone(),
                        lat: after.lat,
                        lon: after.lon,
                        attributes: after.attributes.clone(),
                    };
                    if let Err(e) = ApiHelper::update_device(
                        client,
                        endpoint,
                        project_key,
                        &device_id,
                        &edit_device,
                    )
                    .await
                    {
                        errors.push(format!("更新裝置「{}」失敗: {e}", after.name));
                    }
                }

                for sd in &d.sensor_diffs {
                    if let Some(reason) = &sd.invalid_reason {
                        errors.push(format!("感測器已略過（{reason}）"));
                        continue;
                    }
                    match sd.kind {
                        ChangeKind::Create => {
                            if let Some(s) = &sd.after {
                                if let Err(e) = ApiHelper::create_sensor(
                                    client,
                                    endpoint,
                                    project_key,
                                    &device_id,
                                    s,
                                )
                                .await
                                {
                                    errors.push(format!("新增感測器「{}」失敗: {e}", s.id));
                                }
                            }
                        }
                        ChangeKind::Update => {
                            if let Some(s) = &sd.after {
                                let edit_sensor = EditSensor {
                                    name: s.name.clone(),
                                    desc: s.desc.clone(),
                                    kind: s.kind,
                                    uri: s.uri.clone(),
                                    formula: s.formula.clone(),
                                    attributes: s.attributes.clone(),
                                };
                                if let Err(e) = ApiHelper::update_sensor(
                                    client,
                                    endpoint,
                                    project_key,
                                    &device_id,
                                    &sd.id,
                                    &edit_sensor,
                                )
                                .await
                                {
                                    errors.push(format!("更新感測器「{}」失敗: {e}", sd.id));
                                }
                            }
                        }
                        ChangeKind::Delete => {
                            if let Err(e) = ApiHelper::delete_sensor(
                                client,
                                endpoint,
                                project_key,
                                &device_id,
                                &sd.id,
                            )
                            .await
                            {
                                errors.push(format!("刪除感測器「{}」失敗: {e}", sd.id));
                            }
                        }
                    }
                }
            }
            ChangeKind::Delete => {
                if let Err(e) = ApiHelper::delete_device(client, endpoint, project_key, &d.id).await
                {
                    errors.push(format!("刪除裝置「{}」失敗: {e}", d.id));
                }
            }
        }
    }

    errors
}

#[component]
fn ImportSettingsDialog(
    mut open: Signal<bool>,
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    project_meta: ReadSignal<Vec<Device>>,
    mut project_resource: Resource<Result<Vec<Device>>>,
) -> Element {
    let mut step = use_signal(ImportStep::default);
    let mut parse_error = use_signal(String::new);
    let mut plan = use_signal(Vec::<DeviceDiff>::new);
    let mut is_executing = use_signal(|| false);
    let has_invalid = use_memo(move || {
        plan()
            .iter()
            .any(|d| d.sensor_diffs.iter().any(|sd| sd.invalid_reason.is_some()))
    });

    let mut close_and_reset = move |v: bool| {
        open.set(v);
        if !v {
            step.set(ImportStep::Upload);
            parse_error.set(String::new());
            plan.set(Vec::new());
        }
    };

    let on_file_input = move |e: FormEvent| async move {
        parse_error.set(String::new());
        let files = e.files().clone();
        let Some(file) = files.get(0) else {
            return;
        };
        let Ok(bytes) = file.read_bytes().await else {
            parse_error.set("無法讀取檔案".to_string());
            return;
        };
        let Ok(text) = String::from_utf8(bytes.to_vec()) else {
            parse_error.set("檔案編碼錯誤".to_string());
            return;
        };
        match serde_json::from_str::<Vec<Device>>(&text) {
            Ok(devices) => {
                let computed_plan = compute_import_plan(&project_meta(), &devices);
                plan.set(computed_plan);
                step.set(ImportStep::Preview);
            }
            Err(e) => parse_error.set(format!("JSON 解析失敗: {e}")),
        }
    };

    let on_confirm = move |_| async move {
        is_executing.set(true);
        let toast_api = use_toast();
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let current_plan = plan();
        let errors = execute_import_plan(&client, &ep, &pk, &current_plan).await;
        is_executing.set(false);

        if errors.is_empty() {
            toast_api.success(
                "匯入完成".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
        } else {
            toast_api.error(
                format!("匯入完成，但有 {} 項失敗", errors.len()),
                ToastOptions::new()
                    .description(errors.join("; "))
                    .duration(Duration::from_secs(15)),
            );
        }
        project_resource.restart();
        close_and_reset(false);
    };

    rsx! {
        Dialog {
            open: open(),
            on_open_change: close_and_reset,
            class: "max-w-4xl! max-h-[85vh]! overflow-hidden!",
            DialogTitle { "匯入專案設定" }
            match step() {
                ImportStep::Upload => rsx! {
                    DialogDescription {
                        div { class: "flex flex-col gap-4 text-left",
                            p {
                                "請選擇先前匯出的設定檔（JSON），系統會比對目前的裝置與感測器並列出異動清單。"
                            }
                            div { class: "relative w-full border-2 border-dashed border-slate-200 dark:border-zinc-800 rounded-lg p-6 flex flex-col items-center gap-2 cursor-pointer",
                                div { class: "text-3xl pointer-events-none",
                                    Icon { icon: fa_solid_icons::FaCloudArrowUp }
                                }
                                p { class: "text-sm text-center pointer-events-none", "點擊選取檔案" }
                                input {
                                    class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                    r#type: "file",
                                    accept: "application/json",
                                    oninput: on_file_input,
                                }
                            }
                            if !parse_error().is_empty() {
                                p { class: "text-red-600 dark:text-red-400 text-sm", "{parse_error()}" }
                            }
                        }
                    }
                },
                ImportStep::Preview => rsx! {
                    div { class: "flex flex-col gap-3 text-left flex-1 min-h-0 overflow-y-auto",
                        if plan().is_empty() {
                            p { "沒有偵測到任何異動" }
                        }
                        if has_invalid() {
                            p { class: "text-red-600 dark:text-red-400 text-sm",
                                "有感測器 ID 為空白，請修正檔案後重新上傳"
                            }
                        }
                        for (i , d) in plan().iter().enumerate() {
                            DeviceDiffCard { key: "{i}-{d.id}", diff: d.clone() }
                        }
                    }
                    div { class: "flex justify-end gap-4 mt-4 shrink-0",
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: is_executing(),
                            onclick: move |_| step.set(ImportStep::Upload),
                            "上一步"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_executing() || plan().is_empty() || has_invalid(),
                            onclick: on_confirm,
                            if is_executing() {
                                "執行中..."
                            } else {
                                "確認匯入"
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn DeviceDiffCard(diff: DeviceDiff) -> Element {
    let (label, class) = diff.kind.label_and_class();
    let name = diff
        .after
        .as_ref()
        .or(diff.before.as_ref())
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let id_display = if diff.id.is_empty() {
        "（新裝置，ID 由系統配發）".to_string()
    } else {
        format!("ID: {}", diff.id)
    };
    let changed_fields = match (&diff.before, &diff.after) {
        (Some(b), Some(a)) => device_changed_fields(b, a),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "border border-slate-200 dark:border-zinc-800 rounded-lg p-3",
            div { class: "flex items-center gap-2 flex-wrap",
                span { class: "px-2 py-0.5 border rounded text-xs shrink-0 {class}",
                    "{label}"
                }
                span { class: "font-semibold", "{name}" }
                span { class: "text-xs text-slate-500 dark:text-slate-400", "{id_display}" }
            }
            if !changed_fields.is_empty() {
                p { class: "text-xs text-slate-500 dark:text-slate-400 mt-1",
                    "變更欄位: {changed_fields.join(\", \")}"
                }
            }
            if !diff.sensor_diffs.is_empty() {
                div { class: "flex flex-col gap-1 mt-2 pl-4 border-l-2 border-slate-200 dark:border-zinc-800",
                    for (i , sd) in diff.sensor_diffs.iter().enumerate() {
                        SensorDiffRow { key: "{i}-{sd.id}", diff: sd.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn SensorDiffRow(diff: SensorDiff) -> Element {
    let (label, class) = diff.kind.label_and_class();
    let name = diff
        .after
        .as_ref()
        .or(diff.before.as_ref())
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let id_display = if diff.id.is_empty() {
        "新感測器".to_string()
    } else {
        diff.id.clone()
    };
    let changed_fields = match (&diff.before, &diff.after) {
        (Some(b), Some(a)) => sensor_changed_fields(b, a),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "flex items-center gap-2 text-sm flex-wrap",
            span { class: "px-1.5 py-0.5 border rounded text-xs shrink-0 {class}", "{label}" }
            span { "{id_display}" }
            span { class: "text-slate-500 dark:text-slate-400", "({name})" }
            if !changed_fields.is_empty() {
                span { class: "text-xs text-slate-400", "變更: {changed_fields.join(\", \")}" }
            }
            if let Some(reason) = &diff.invalid_reason {
                span { class: "px-1.5 py-0.5 border rounded text-xs shrink-0 text-red-600 dark:text-red-400 border-red-600 dark:border-red-400",
                    "{reason}"
                }
            }
        }
    }
}

// ─── Helper: Device Header Card ──────────────────────────────────────────────

#[component]
fn DeviceHeaderCard(project_name: ReadSignal<String>, device: ReadSignal<Device>) -> Element {
    let desc = device().desc.unwrap_or_default();
    let nav = navigator();
    let back = move |_| {
        nav.push(Route::ProjectDevices {
            project_name: project_name(),
        });
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle { {device().name} }
                CardDescription { {desc} }
                CardAction {
                    Button { variant: ButtonVariant::Ghost, onclick: back,
                        Icon { icon: fa_solid_icons::FaArrowLeft }
                    }
                }
            }
            CardContent {
                p { {device().id} }
            }
        }
    }
}

// ─── Helper: Sensor Header Card ─────────────────────────────────────────────

#[component]
fn SensorHeaderCard(
    project_name: ReadSignal<String>,
    device_id: ReadSignal<String>,
    sensor: ReadSignal<Sensor>,
) -> Element {
    let desc = sensor().desc.unwrap_or_default();
    let nav = navigator();
    let back = move |_| {
        nav.push(Route::DeviceSensors {
            project_name: project_name(),
            device_id: device_id(),
        });
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle { {sensor().name} }
                CardDescription { {desc} }
                CardAction {
                    Button { variant: ButtonVariant::Ghost, onclick: back,
                        Icon { icon: fa_solid_icons::FaArrowLeft }
                    }
                }
            }
            CardContent {
                p { {sensor().id} }
            }
        }
    }
}

// ─── DeviceSensors (sensor list for a device) ────────────────────────────────

#[component]
pub fn DeviceSensors(project_name: ReadSignal<String>, device_id: ReadSignal<String>) -> Element {
    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();

    let device = use_memo(move || project_meta().into_iter().find(|d| d.id == device_id()));

    let Some(device_val) = device() else {
        return rsx! {
            p { "Device not found" }
        };
    };

    let device_name = device_val.name.clone();
    use_effect(move || {
        let title = format!("{} - {}", project_name(), device_name);
        consume_context::<HeaderContext>().set_title(&title);
    });

    let mut device_signal = use_signal(move || device_val.clone());
    // Keep device_signal in sync when project_meta updates
    use_effect(move || {
        if let Some(d) = device() {
            device_signal.set(d);
        }
    });

    let mut timer = use_signal(|| 10);
    let mut resource: Resource<Result<Vec<SensorWithData>, Error>> =
        use_resource(move || async move {
            let device_id = device_id();
            let project_key = project().project_key;
            let client = reqwest::Client::new();

            let sensors = device_signal().sensors.unwrap_or_default();
            let raw_datas =
                ApiHelper::fetch_raw_data(&client, &endpoint(), &device_id, &project_key).await?;
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

    // Create sensor dialog
    let mut new_sensor = use_store(|| Sensor::new());
    let mut new_sensor_open = use_signal(|| false);

    let on_create_sensor = move |_| async move {
        let toast_api = use_toast();
        if new_sensor.id()().trim().is_empty() {
            toast_api.error(
                "建立失敗".to_string(),
                ToastOptions::new()
                    .description("感測器 ID 不可為空白")
                    .duration(Duration::from_secs(10)),
            );
            return;
        }
        new_sensor_open.set(false);
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let did = device_id();
        let ret = ApiHelper::create_sensor(&client, &ep, &pk, &did, &new_sensor()).await;
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

    let radio_items = SensorType::iter().enumerate().map(|(i, t)| {
        rsx! {
            RadioItem { index: i, value: serde_json::to_string(&t).unwrap(), {t.to_string()} }
        }
    });

    let new_sensor_dialog = rsx! {
        Dialog {
            open: new_sensor_open(),
            on_open_change: move |v| new_sensor_open.set(v),
            DialogTitle { "New Sensor" }
            div { class: "grid grid-cols-1 gap-4",
                Label { html_for: "new_sensor_id", "ID *" }
                DxInput {
                    id: "new_sensor_id",
                    required: true,
                    value: new_sensor.id(),
                    oninput: move |e: FormEvent| new_sensor.id().set(e.value()),
                }
                Label { html_for: "new_sensor_name", "Name" }
                DxInput {
                    id: "new_sensor_name",
                    value: new_sensor.name(),
                    oninput: move |e: FormEvent| new_sensor.name().set(e.value()),
                }
                Label { html_for: "new_sensor_desc", "Description" }
                Textarea {
                    id: "new_sensor_desc",
                    value: new_sensor.desc(),
                    oninput: move |e: FormEvent| new_sensor.desc().set(Some(e.value())),
                }
                Label { html_for: "new_sensor_type", "Type" }
                RadioGroup {
                    id: "new_sensor_type",
                    on_value_change: move |v: String| {
                        if let Ok(v) = serde_json::from_str(&v) {
                            new_sensor.kind().set(v);
                        }
                    },
                    value: serde_json::to_string(&*new_sensor.kind().read()).unwrap(),
                    {radio_items}
                }
                Button { onclick: on_create_sensor, "Create" }
            }
        }
    };

    // Delete sensor dialog
    let mut delete_ctx = use_store(|| DeleteCtx {
        is_open: false,
        target: String::new(),
    });

    let on_delete_confirm = move |_| async move {
        let toast_api = use_toast();
        let target = delete_ctx.target()();
        delete_ctx.write().is_open = false;
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let did = device_id();
        let ret = ApiHelper::delete_sensor(&client, &ep, &pk, &did, &target).await;
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
        Dialog {
            open: *delete_ctx.is_open().read(),
            on_open_change: move |v| delete_ctx.is_open().set(v),
            DialogTitle { "Delete Sensor" }
            DialogDescription {
                div { class: "flex flex-col gap-4",
                    "Delete sensor {delete_ctx.target()}"
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
    };

    rsx! {
        Breadcrumb {
            items: vec![
                BreadcrumbItem::link("Projects", Route::ProjectsView {}),
                BreadcrumbItem::link(
                    project_name(),
                    Route::ProjectDevices {
                        project_name: project_name(),
                    },
                ),
            ],
        }
        PageHeader { title: device_signal().name,
            Button {
                variant: ButtonVariant::Ghost,
                onclick: move |_| {
                    resource.restart();
                    timer.set(10);
                },
                "Refresh: {timer()}"
            }
            Button {
                onclick: move |_| {
                    new_sensor.set(Sensor::new());
                    new_sensor_open.set(true);
                },
                "Add Sensor"
            }
        }
        {new_sensor_dialog}
        {delete_dialog}
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
            if let Some(response) = &*resource.read() {
                match response {
                    Ok(sensors) => rsx! {
                        if sensors.is_empty() {
                            p { "No sensor, Add new one" }
                        }
                        for s in sensors {
                            SensorCard {
                                key: "{s.sensor.id}",
                                project_name,
                                device_id,
                                project,
                                endpoint,
                                sensor_data: s.clone(),
                                delete_ctx,
                            }
                        }
                    },
                    Err(err) => rsx! { "Failed to fetch response: {err}" },
                }
            } else {
                "Loading..."
            }
        }
        div { class: "h-48" }
    }
}

// ─── SensorCard (individual sensor in the grid) ─────────────────────────────

#[component]
fn SensorCard(
    project_name: ReadSignal<String>,
    device_id: ReadSignal<String>,
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    sensor_data: ReadSignal<SensorWithData>,
    delete_ctx: Store<DeleteCtx>,
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
            let device_id = device_id();
            let project_key = project().project_key;
            let snapshot_id = first_value[11..].to_string();
            let img_b64 = ApiHelper::fetch_snapshot_base64(
                &client,
                &endpoint(),
                &device_id,
                &sensor_id,
                &snapshot_id,
                &project_key,
            )
            .await?;
            Ok(img_b64)
        } else {
            Err(anyhow!("Not a snapshot"))
        }
    });

    let sensor_id = use_memo(move || sensor().id.clone());

    let nav1 = navigator();
    let attr_click = move |_| {
        nav1.push(Route::SensorAttr {
            project_name: project_name(),
            device_id: device_id(),
            sensor_id: sensor_id(),
        });
    };
    let nav2 = navigator();
    let history_click = move |_| {
        nav2.push(Route::SensorHistory {
            project_name: project_name(),
            device_id: device_id(),
            sensor_id: sensor_id(),
        });
    };
    let delete_click = move |_| delete_ctx.prompt_delete(&sensor_id());

    rsx! {
        Card {
            CardHeader {
                CardTitle { {sensor().name} }
                CardAction {
                    DropdownMenu {
                        DxDropdownMenuTrigger {
                            Icon { icon: fa_solid_icons::FaEllipsisVertical }
                        }
                        DxDropdownMenuContent {
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
                                value: "Setting".to_string(),
                                index: 1usize,
                                on_select: attr_click,
                                div { class: "flex gap-2",
                                    Icon { icon: fa_solid_icons::FaGear }
                                    "Setting"
                                }
                            }
                            DropdownMenuItem::<String> {
                                value: "Delete".to_string(),
                                index: 2usize,
                                on_select: delete_click,
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
            CardFooter {
                div { class: "flex flex-col",
                    p { {data().map(|d| d.time).unwrap_or_default()} }
                    p { {sensor().id} }
                }
            }
        }
    }
}

// ─── DeviceAttr (device info + attributes + monitor) ─────────────────────────

#[component]
pub fn DeviceAttr(project_name: ReadSignal<String>, device_id: ReadSignal<String>) -> Element {
    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();

    let device = use_memo(move || project_meta().into_iter().find(|d| d.id == device_id()));

    let Some(device_val) = device() else {
        return rsx! {
            p { "Device not found" }
        };
    };

    let mut device_signal = use_signal(move || device_val);
    use_effect(move || {
        if let Some(d) = device() {
            device_signal.set(d);
        }
    });

    let mut attributes = use_signal(move || device_signal().attributes.unwrap_or_default());
    let is_dirty = use_memo(move || attributes() != device_signal().attributes.unwrap_or_default());

    let mut device_info = use_signal(move || device_signal().clone());
    let is_device_dirty = use_memo(move || {
        device_info().name != device_signal().name || device_info().desc != device_signal().desc
    });

    let save_attrs = move |_| async move {
        let edit_device = EditDevice {
            name: device_signal().name.clone(),
            kind: device_signal().kind.clone(),
            attributes: Some(attributes().clone()),
            ..Default::default()
        };
        let client = Client::new();
        let toastapi = use_toast();

        let result = ApiHelper::update_device(
            &client,
            &endpoint(),
            &project().project_key,
            &device_id(),
            &edit_device,
        )
        .await;

        match result {
            Ok(text) => {
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                project_resource.restart();
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
            kind: device_signal().kind.clone(),
            desc: device_info().desc.clone(),
            ..Default::default()
        };
        let client = Client::new();
        let toastapi = use_toast();

        let result = ApiHelper::update_device(
            &client,
            &endpoint(),
            &project().project_key,
            &device_id(),
            &edit_device,
        )
        .await;

        match result {
            Ok(text) => {
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(5)),
                );
                project_resource.restart();
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
        Breadcrumb {
            items: vec![
                BreadcrumbItem::link("Projects", Route::ProjectsView {}),
                BreadcrumbItem::link(
                    project_name(),
                    Route::ProjectDevices {
                        project_name: project_name(),
                    },
                ),
            ],
        }
        PageHeader { title: device_signal().name }

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
            div { {device_signal().id} }
            div { "Type" }
            div { {device_signal().kind} }
            div { "Name" }
            div {
                DxInput {
                    class: "w-full",
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
            if attributes().is_empty() {
                p { "No attributes" }
            }
            for (i , attr) in attributes().iter().enumerate() {
                div { class: "flex gap-4 mb-8",
                    div { class: "flex flex-1 gap-4 flex-wrap",
                        DxInput {
                            class: "flex-1",
                            placeholder: "Key",
                            onchange: move |e: FormEvent| {
                                attributes.write()[i].key = e.value();
                            },
                            value: attr.key.clone(),
                        }
                        DxInput {
                            class: "flex-1",
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

        MonitorPanel { project, endpoint, device: device_signal }
        div { class: "h-48" }
    }
}

// ─── MonitorPanel (active monitor status/settings/notifications) ─────────────

#[component]
fn MonitorPanel(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
) -> Element {
    let active_status: Resource<Result<Option<ActiveInfo>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        ApiHelper::fetch_active_info(&client, &endpoint(), &device().id, &project().project_key)
            .await
    });

    let active_setting: Resource<Result<ActiveDevice>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        ApiHelper::fetch_active_setting(&client, &endpoint(), &device().id, &project().project_key)
            .await
    });

    let active_notify: Resource<Result<Vec<ActiveNotify>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        ApiHelper::fetch_active_notifies(&client, &endpoint(), &device().id, &project().project_key)
            .await
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
            Ok(setting) => rsx! {
                ActiveSettingSection {
                    project,
                    endpoint,
                    device,
                    setting: setting.clone(),
                    active_setting,
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
fn ActiveSettingSection(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    setting: ReadSignal<ActiveDevice>,
    active_setting: Resource<Result<ActiveDevice, Error>>,
) -> Element {
    let mut setting_clone = use_signal(|| setting().clone());
    use_effect(move || {
        setting_clone.set(setting());
    });
    let is_dirty = use_memo(move || setting() != setting_clone());
    let is_dirty_variant = use_memo(move || {
        if is_dirty() {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Secondary
        }
    });

    let save_active_setting = move |_| async move {
        let toastapi = use_toast();
        let client = reqwest::Client::new();
        let edit_active = ActiveDevice {
            device_id: device().id.clone(),
            enable: setting_clone().enable,
            period: setting_clone().period.clone(),
            min_uploads: setting_clone().min_uploads.clone(),
            max_uploads: setting_clone().max_uploads.clone(),
            sensor: setting_clone().sensor.clone(),
            create_time: setting().create_time,
        };
        let result = ApiHelper::update_active_setting(
            &client,
            &endpoint(),
            &project().project_key,
            &device().id,
            &edit_active,
        )
        .await;

        match result {
            Ok(text) => {
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
            SelectOption::<Option<String>> { index: i + 1, value: "{s.id}", text_value: "{s.id}", "{s.id}" }
        }
    });

    rsx! {
        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Active Setting" }
            Button { variant: is_dirty_variant(), onclick: save_active_setting, "Save" }
        }
        div { class: "grid grid-cols-[auto_auto] gap-2",
            div { "Device ID:" }
            div { {setting().device_id.clone()} }
            div { "Enabled:" }
            div { class: "flex items-center gap-4",
                Switch {
                    checked: setting_clone().enable,
                    on_checked_change: move |c| setting_clone.write().enable = c,
                }
                "{setting_clone().enable}"
            }
            div { "Period:" }
            div {
                DxInput {
                    value: setting_clone().period,
                    onchange: move |e: FormEvent| setting_clone.write().period = e.value(),
                }
            }
            div { "Min Uploads:" }
            div {
                DxInput {
                    value: setting_clone().min_uploads.map(|v| v.to_string()),
                    onchange: move |e: FormEvent| {
                        if e.value().is_empty() {
                            setting_clone.write().min_uploads = None;
                        } else if let Ok(value) = e.value().parse::<i32>() {
                            setting_clone.write().min_uploads = Some(value);
                        }
                    },
                }
            }
            div { "Max Uploads:" }
            div {
                DxInput {
                    value: setting_clone().max_uploads.map(|v| v.to_string()),
                    onchange: move |e: FormEvent| {
                        if e.value().is_empty() {
                            setting_clone.write().max_uploads = None;
                        } else if let Ok(value) = e.value().parse::<i32>() {
                            setting_clone.write().max_uploads = Some(value);
                        }
                    },
                }
            }
            div { "Sensor" }
            div {
                Select::<Option<String>> {
                    default_value: setting_clone().sensor,
                    on_value_change: move |e: Option<Option<String>>| {
                        setting_clone.write().sensor = e.unwrap_or_default()
                    },
                    SelectGroup {
                        SelectOption::<Option<String>> {
                            index: 0usize,
                            value: None,
                            text_value: "(All Sensor)",
                            "(All Sensor)"
                        }
                        {sensor_select}
                    }
                }
            }
            div { "Created:" }
            div { "{setting().create_time.unwrap_or_default()}" }
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

        let result = ApiHelper::upsert_active_notify(
            &client,
            &endpoint(),
            &project().project_key,
            &device().id,
            &new_notify,
        )
        .await;

        match result {
            Ok((_, text)) => {
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
fn ActiveNotifyCard(
    project: ReadSignal<Project>,
    endpoint: ReadSignal<Endpoint>,
    device: ReadSignal<Device>,
    notify: ReadSignal<ActiveNotify>,
    active_notify: Resource<Result<Vec<ActiveNotify>>>,
) -> Element {
    let mut notify_edit = use_signal(move || notify().clone());
    let is_dirty_variant = use_memo(move || {
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

        let result = ApiHelper::upsert_active_notify(
            &client,
            &endpoint(),
            &project().project_key,
            &device().id,
            &edit_notify,
        )
        .await;

        match result {
            Ok((status, text)) => {
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

        let _ret = ApiHelper::delete_active_notify(
            &client,
            &endpoint(),
            &project().project_key,
            &device().id,
            notify_edit().id,
        )
        .await;
        active_notify.restart();
    };

    let on_delete_click = move |_| async move {
        delete_dialog_open.set(false);
        let client = reqwest::Client::new();
        let toastapi = use_toast();
        let ret = ApiHelper::delete_active_notify(
            &client,
            &endpoint(),
            &project().project_key,
            &device().id,
            notify().id,
        )
        .await;

        match ret {
            Ok(text) => {
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
                    "Delete Failed".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
            }
        }
    };

    let delete_dialog = rsx! {
        Dialog {
            open: delete_dialog_open(),
            on_open_change: move |v| delete_dialog_open.set(v),
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
    };

    rsx! {
        {delete_dialog}
        div { class: "border rounded-lg p-4 relative",
            div { class: "absolute top-2 right-2 flex space-x-2",
                Button { variant: is_dirty_variant(), onclick: on_save_click, "Save" }
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
                    DxInput {
                        value: notify_edit().name,
                        oninput: move |e: FormEvent| notify_edit.write().name = e.value(),
                    }
                }
                div { class: "font-semibold", "Type:" }
                div {
                    DxInput {
                        value: notify_edit().kind,
                        oninput: move |e: FormEvent| notify_edit.write().kind = e.value(),
                    }
                }
                div { class: "font-semibold", "Enabled:" }
                div { class: "flex items-center gap-4",
                    Switch {
                        checked: notify_edit().enable,
                        on_checked_change: move |v| notify_edit.write().enable = v,
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

// ─── SensorAttr (sensor attributes) ─────────────────────────────────────────

#[component]
pub fn SensorAttr(
    project_name: ReadSignal<String>,
    device_id: ReadSignal<String>,
    sensor_id: ReadSignal<String>,
) -> Element {
    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();

    let device = use_memo(move || project_meta().into_iter().find(|d| d.id == device_id()));
    let sensor = use_memo(move || {
        device()
            .and_then(|d| d.sensors)
            .and_then(|sensors| sensors.into_iter().find(|s| s.id == sensor_id()))
    });

    let (Some(_device_val), Some(sensor_val)) = (device(), sensor()) else {
        return rsx! {
            p { "Sensor not found" }
        };
    };

    let mut sensor_signal = use_signal(move || sensor_val);
    use_effect(move || {
        if let Some(s) = sensor() {
            sensor_signal.set(s);
        }
    });

    let mut sensor_info = use_signal(move || sensor_signal());
    let is_sensor_info_dirty = use_memo(move || {
        sensor_info().name != sensor_signal().name
            || sensor_info().desc != sensor_signal().desc
            || sensor_info().kind != sensor_signal().kind
            || sensor_info().uri != sensor_signal().uri
            || sensor_info().formula != sensor_signal().formula
    });

    let save_sensor_info = move |_| async move {
        let edit_sensor = EditSensor {
            name: sensor_info().name.clone(),
            desc: sensor_info().desc.clone(),
            kind: sensor_info().kind,
            uri: sensor_info().uri.clone(),
            formula: sensor_info().formula.clone(),
            ..Default::default()
        };
        let client = Client::new();
        let toastapi = use_toast();

        let result = ApiHelper::update_sensor(
            &client,
            &endpoint(),
            &project().project_key,
            &device_id(),
            &sensor_id(),
            &edit_sensor,
        )
        .await;

        match result {
            Ok(text) => {
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(10)),
                );
                project_resource.restart();
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

    let mut attributes = use_signal(move || sensor_signal().attributes.unwrap_or_default());
    let is_dirty = use_memo(move || attributes() != sensor_signal().attributes.unwrap_or_default());

    let save_attrs = move |_| async move {
        let edit_sensor = EditSensor {
            name: sensor_signal().name.clone(),
            kind: sensor_signal().kind.clone(),
            attributes: Some(attributes().clone()),
            ..Default::default()
        };
        let client = Client::new();
        let toastapi = use_toast();

        let result = ApiHelper::update_sensor(
            &client,
            &endpoint(),
            &project().project_key,
            &device_id(),
            &sensor_id(),
            &edit_sensor,
        )
        .await;

        match result {
            Ok(text) => {
                toastapi.success(
                    "Updated".to_string(),
                    ToastOptions::new()
                        .description(text)
                        .duration(Duration::from_secs(10)),
                );
                project_resource.restart();
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

    let sensor_type_items = SensorType::iter().enumerate().map(|(i, t)| {
        rsx! {
            RadioItem { index: i, value: serde_json::to_string(&t).unwrap(), {t.to_string()} }
        }
    });

    rsx! {
        Breadcrumb {
            items: vec![
                BreadcrumbItem::link("Projects", Route::ProjectsView {}),
                BreadcrumbItem::link(
                    project_name(),
                    Route::ProjectDevices {
                        project_name: project_name(),
                    },
                ),
                BreadcrumbItem::link(
                    device().map(|d| d.name).unwrap_or_default(),
                    Route::DeviceSensors {
                        project_name: project_name(),
                        device_id: device_id(),
                    },
                ),
            ],
        }
        PageHeader { title: sensor_signal().name }

        div { class: "grid grid-cols-[1fr_auto] items-center mt-8",
            h1 { class: "text-2xl font-bold", "Sensor Info" }
            Button {
                variant: if is_sensor_info_dirty() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                onclick: save_sensor_info,
                "Save"
            }
        }
        div { class: "grid grid-cols-[auto_auto] gap-2 w-full",
            div { "ID" }
            div { {sensor_signal().id} }
            div { "Type" }
            div {
                RadioGroup {
                    on_value_change: move |v: String| {
                        if let Ok(v) = serde_json::from_str(&v) {
                            sensor_info.write().kind = v;
                        }
                    },
                    value: serde_json::to_string(&sensor_info().kind).unwrap(),
                    {sensor_type_items}
                }
            }
            div { "Name" }
            div {
                DxInput {
                    class: "w-full",
                    oninput: move |i: FormEvent| { sensor_info.write().name = i.value() },
                    value: sensor_info().name,
                }
            }
            div { "Desc" }
            div {
                Textarea {
                    oninput: move |i: FormEvent| { sensor_info.write().desc = Some(i.value()) },
                    value: sensor_info().desc,
                }
            }
            div { "URI" }
            div {
                DxInput {
                    class: "w-full",
                    oninput: move |i: FormEvent| { sensor_info.write().uri = Some(i.value()) },
                    value: sensor_info().uri,
                }
            }
            div { "Formula" }
            div {
                Textarea {
                    oninput: move |i: FormEvent| { sensor_info.write().formula = Some(i.value()) },
                    value: sensor_info().formula,
                }
            }
        }

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
                if attributes().is_empty() {
                    p { "No attributes" }
                }

                for (i , attr) in attributes().iter().enumerate() {
                    div { class: "flex gap-4 mb-8",
                        div { class: "flex flex-1 gap-4 flex-wrap",
                            DxInput {
                                class: "flex-1",
                                placeholder: "Key",
                                onchange: move |e: FormEvent| {
                                    attributes.write()[i].key = e.value();
                                },
                                value: attr.key.clone(),
                            }
                            DxInput {
                                class: "flex-1",
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
        div { class: "h-48" }
    }
}

// ─── SensorHistory (raw data history table) ──────────────────────────────────

#[component]
pub fn SensorHistory(
    project_name: ReadSignal<String>,
    device_id: ReadSignal<String>,
    sensor_id: ReadSignal<String>,
) -> Element {
    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;

    let device = use_memo(move || project_meta().into_iter().find(|d| d.id == device_id()));
    let sensor = use_memo(move || {
        device()
            .and_then(|d| d.sensors)
            .and_then(|sensors| sensors.into_iter().find(|s| s.id == sensor_id()))
    });

    let (Some(_device_val), Some(sensor_val)) = (device(), sensor()) else {
        return rsx! {
            p { "Sensor not found" }
        };
    };

    let mut sensor_signal = use_signal(move || sensor_val);
    use_effect(move || {
        if let Some(s) = sensor() {
            sensor_signal.set(s);
        }
    });

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
        let ret = ApiHelper::fetch_sensor_raw_data(
            &client,
            &endpoint(),
            SensorRawDataQuery {
                device_id: &device_id(),
                sensor_id: &sensor_id(),
                project_key: &project().project_key,
                start: &start_str,
                end: &end_str,
                order: asc_or_desc,
            },
        )
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
        if sensor_signal().kind != SensorType::Snapshot {
            return Ok(None::<String>);
        }
        let mut snapshot_url: Option<String> = None;
        if let Some(Ok(data)) = &*raw_datas.read() {
            if let Some(sel_idx) = selected_index() {
                if sel_idx < data.len() {
                    let raw_data = &data[sel_idx];
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
            let sid = sensor_id();
            let did = device_id();
            let project_key = project().project_key;
            let snapshot_id = snapshot_url[11..].to_string();
            let img_b64 = ApiHelper::fetch_snapshot_base64(
                &client,
                &endpoint(),
                &did,
                &sid,
                &snapshot_id,
                &project_key,
            )
            .await?;
            return Ok(Some(img_b64));
        }
        Ok(None)
    });

    let imgdata = use_memo(move || {
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
        Breadcrumb {
            items: vec![
                BreadcrumbItem::link("Projects", Route::ProjectsView {}),
                BreadcrumbItem::link(
                    project_name(),
                    Route::ProjectDevices {
                        project_name: project_name(),
                    },
                ),
                BreadcrumbItem::link(
                    device().map(|d| d.name).unwrap_or_default(),
                    Route::DeviceSensors {
                        project_name: project_name(),
                        device_id: device_id(),
                    },
                ),
            ],
        }
        PageHeader { title: sensor_signal().name }

        h2 { class: "text-xl font-bold mb-4", "Raw Data Records" }

        div { class: "flex justify-center gap-4 flex-wrap items-center",
            div { class: "flex justify-center gap-4",
                p { "GMT+8" }
                Switch {
                    checked: use_utc(),
                    on_checked_change: move |b| use_utc.set(b),
                }
                p { "UTC" }
            }
            div { class: "flex justify-center gap-4",
                p { "DESC" }
                Switch {
                    checked: use_asc(),
                    on_checked_change: move |b| use_asc.set(b),
                }
                p { "ASC" }
            }
            div { class: "flex justify-center gap-4",
                Button { onclick: minus_one_day,
                    Icon { icon: fa_solid_icons::FaChevronLeft }
                }
                DxInput {
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
                DxInput {
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
        div { class: "h-48" }
    }
}

#[component]
fn SensorRawDataTable(
    datas: ReadSignal<Vec<GetRawData>>,
    selected_index: Signal<Option<usize>>,
    imgdata: ReadSignal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "mt-8 mb-8",
            div {
                class: "grid grid-cols-1 gap-4",
                class: if imgdata().is_some() { "md:grid-cols-2" },
                div { class: "flex flex-col overflow-auto max-h-[60lvh]",
                    table { class: "border-collapse border border-(--primary-color-7)",
                        thead {
                            tr { class: "bg-(--primary-color-5)",
                                th { class: "border border-(--primary-color-7) p-2 text-left text-(--secondary-color-1)",
                                    "Time"
                                }
                                th { class: "border border-(--primary-color-7) p-2 text-left text-(--secondary-color-1)",
                                    "Value"
                                }
                            }
                        }
                        tbody {
                            for (index , data) in datas().iter().enumerate() {
                                tr {
                                    key: "{data.time.clone()}",
                                    class: "hover:bg-(--primary-color-5) cursor-pointer text-(--secondary-color-1)",
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
