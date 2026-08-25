use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use futures_util::future::join_all;
use reqwest::Client;

use crate::{
    api::ApiHelper,
    components::{
        button::{Button, ButtonVariant},
        checkbox::Checkbox,
        label::Label,
        select::{Select, SelectGroup, SelectOption},
        switch::Switch,
    },
    models::{ActiveDevice, Device, Endpoint},
    ui::{
        breadcrumb::{Breadcrumb, BreadcrumbItem},
        custom::DxInput,
        page_header::PageHeader,
    },
    views::{global::HeaderContext, sensor_v2::ProjectContext},
    Route,
};

#[derive(Clone, PartialEq)]
struct ActiveSettingRow {
    device: Device,
    setting: Option<ActiveDevice>,
    error: Option<String>,
}

#[derive(Clone, PartialEq)]
struct SettingDraft {
    setting: ActiveDevice,
    min_uploads_text: String,
    max_uploads_text: String,
    min_uploads_valid: bool,
    max_uploads_valid: bool,
}

impl Default for SettingDraft {
    fn default() -> Self {
        Self::from_setting(&ActiveDevice::default())
    }
}

impl SettingDraft {
    fn from_setting(setting: &ActiveDevice) -> Self {
        Self {
            setting: setting.clone(),
            min_uploads_text: setting
                .min_uploads
                .map(|value| value.to_string())
                .unwrap_or_default(),
            max_uploads_text: setting
                .max_uploads
                .map(|value| value.to_string())
                .unwrap_or_default(),
            min_uploads_valid: true,
            max_uploads_valid: true,
        }
    }

    fn is_valid(&self) -> bool {
        self.min_uploads_valid && self.max_uploads_valid
    }

    fn for_device(&self, device: &Device, current: &ActiveDevice) -> Self {
        let mut draft = self.clone();
        draft.setting.device_id = device.id.clone();
        draft.setting.create_time = current.create_time;
        draft
    }
}

#[derive(Clone, PartialEq)]
struct ActiveSettingFieldDiff {
    label: &'static str,
    before: String,
    after: String,
}

#[derive(Clone, PartialEq)]
struct ActiveSettingDiff {
    device_id: String,
    device_name: String,
    before: ActiveDevice,
    after: ActiveDevice,
    fields: Vec<ActiveSettingFieldDiff>,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum PageStep {
    #[default]
    Edit,
    Preview,
}

async fn fetch_active_setting_rows(
    client: &Client,
    endpoint: &Endpoint,
    project_key: &str,
    devices: Vec<Device>,
) -> Result<Vec<ActiveSettingRow>> {
    let requests = devices.into_iter().map(|device| {
        let device_id = device.id.clone();
        async move {
            match ApiHelper::fetch_active_setting(client, endpoint, &device_id, project_key).await {
                Ok(setting) => ActiveSettingRow {
                    device,
                    setting: Some(setting),
                    error: None,
                },
                Err(error) => ActiveSettingRow {
                    device,
                    setting: None,
                    error: Some(error.to_string()),
                },
            }
        }
    });

    Ok(join_all(requests).await)
}

fn parse_optional_i32(value: &str) -> (Option<i32>, bool) {
    if value.is_empty() {
        (None, true)
    } else {
        match value.parse::<i32>() {
            Ok(value) => (Some(value), true),
            Err(_) => (None, false),
        }
    }
}

fn collect_sensor_ids(devices: &[Device]) -> Vec<String> {
    let mut sensor_ids = HashSet::new();
    for device in devices {
        for sensor in device.sensors.iter().flatten() {
            sensor_ids.insert(sensor.id.clone());
        }
    }
    let mut sensor_ids: Vec<_> = sensor_ids.into_iter().collect();
    sensor_ids.sort();
    sensor_ids
}

fn device_has_sensor(device: &Device, sensor_id: &str) -> bool {
    device
        .sensors
        .iter()
        .flatten()
        .any(|sensor| sensor.id == sensor_id)
}

fn setting_field_labels(before: &ActiveDevice, after: &ActiveDevice) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if before.enable != after.enable {
        fields.push("啟用");
    }
    if before.period != after.period {
        fields.push("週期");
    }
    if before.min_uploads != after.min_uploads {
        fields.push("最小上傳次數");
    }
    if before.max_uploads != after.max_uploads {
        fields.push("最大上傳次數");
    }
    if before.sensor != after.sensor {
        fields.push("Sensor");
    }
    fields
}

fn setting_field_value(setting: &ActiveDevice, label: &str) -> String {
    match label {
        "啟用" => {
            if setting.enable {
                "啟用".to_string()
            } else {
                "停用".to_string()
            }
        }
        "週期" => {
            if setting.period.is_empty() {
                "未設定".to_string()
            } else {
                setting.period.clone()
            }
        }
        "最小上傳次數" => setting
            .min_uploads
            .map(|value| value.to_string())
            .unwrap_or_else(|| "不限".to_string()),
        "最大上傳次數" => setting
            .max_uploads
            .map(|value| value.to_string())
            .unwrap_or_else(|| "不限".to_string()),
        "Sensor" => setting
            .sensor
            .clone()
            .unwrap_or_else(|| "全部感測器".to_string()),
        _ => String::new(),
    }
}

fn build_diffs(
    rows: &[ActiveSettingRow],
    drafts: &HashMap<String, SettingDraft>,
) -> Vec<ActiveSettingDiff> {
    rows.iter()
        .filter_map(|row| {
            let before = row.setting.as_ref()?;
            let draft = drafts.get(&row.device.id)?;
            let after = &draft.setting;
            let labels = setting_field_labels(before, after);
            if labels.is_empty() {
                return None;
            }
            let fields = labels
                .into_iter()
                .map(|label| ActiveSettingFieldDiff {
                    label,
                    before: setting_field_value(before, label),
                    after: setting_field_value(after, label),
                })
                .collect();
            Some(ActiveSettingDiff {
                device_id: row.device.id.clone(),
                device_name: row.device.name.clone(),
                before: before.clone(),
                after: after.clone(),
                fields,
            })
        })
        .collect()
}

fn setting_errors(
    rows: &[ActiveSettingRow],
    drafts: &HashMap<String, SettingDraft>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (device_id, draft) in drafts {
        let Some(row) = rows.iter().find(|row| row.device.id == *device_id) else {
            continue;
        };
        if !draft.min_uploads_valid {
            errors.push(format!(
                "裝置「{}」的最小上傳次數必須是整數",
                row.device.name
            ));
        }
        if !draft.max_uploads_valid {
            errors.push(format!(
                "裝置「{}」的最大上傳次數必須是整數",
                row.device.name
            ));
        }
        if let Some(sensor_id) = &draft.setting.sensor {
            if !device_has_sensor(&row.device, sensor_id) {
                errors.push(format!(
                    "裝置「{}」沒有 Sensor「{}」",
                    row.device.name, sensor_id
                ));
            }
        }
    }
    errors
}

fn batch_sensor_errors(
    rows: &[ActiveSettingRow],
    selected: &HashSet<String>,
    template: &SettingDraft,
) -> Vec<String> {
    let Some(sensor_id) = template.setting.sensor.as_deref() else {
        return Vec::new();
    };

    let mut selected_ids: Vec<_> = selected.iter().cloned().collect();
    selected_ids.sort();
    selected_ids
        .into_iter()
        .filter_map(|device_id| rows.iter().find(|row| row.device.id == device_id))
        .filter(|row| !device_has_sensor(&row.device, sensor_id))
        .map(|row| format!("裝置「{}」沒有 Sensor「{}」", row.device.name, sensor_id))
        .collect()
}

async fn execute_diffs(
    client: &Client,
    endpoint: &Endpoint,
    project_key: &str,
    diffs: &[ActiveSettingDiff],
) -> Vec<String> {
    let mut errors = Vec::new();
    for diff in diffs {
        if let Err(error) = ApiHelper::update_active_setting(
            client,
            endpoint,
            project_key,
            &diff.device_id,
            &diff.after,
        )
        .await
        {
            errors.push(format!(
                "裝置「{}」（{}）更新失敗：{}",
                diff.device_name, diff.device_id, error
            ));
        }
    }
    errors
}

#[component]
pub fn ProjectActiveSettingOverview(project_name: ReadSignal<String>) -> Element {
    use_effect(move || {
        consume_context::<HeaderContext>().set_title("主動監控總覽");
    });

    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = use_context();

    let mut settings: Resource<Result<Vec<ActiveSettingRow>>> = use_resource(move || async move {
        let client = Client::new();
        let project_key = project().project_key;
        fetch_active_setting_rows(&client, &endpoint(), &project_key, project_meta()).await
    });

    let mut drafts: Signal<HashMap<String, SettingDraft>> = use_signal(HashMap::new);
    let mut selected: Signal<HashSet<String>> = use_signal(HashSet::new);
    let mut batch_template = use_signal(SettingDraft::default);
    let mut filter = use_signal(String::new);
    let mut step = use_signal(PageStep::default);
    let mut is_executing = use_signal(|| false);

    let sensor_ids = use_memo(move || collect_sensor_ids(&project_meta()));

    use_effect(move || {
        let _ = project_meta();
        drafts.set(HashMap::new());
        selected.set(HashSet::new());
        batch_template.set(SettingDraft::default());
        step.set(PageStep::Edit);
    });

    let (rows, load_error, is_loading) = {
        let resource_state = settings.read();
        match &*resource_state {
            Some(Ok(rows)) => (rows.clone(), None, false),
            Some(Err(error)) => (Vec::new(), Some(error.to_string()), false),
            None => (Vec::new(), None, true),
        }
    };

    let filter_value = filter().trim().to_lowercase();
    let visible_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            filter_value.is_empty()
                || row.device.name.to_lowercase().contains(&filter_value)
                || row.device.id.to_lowercase().contains(&filter_value)
        })
        .cloned()
        .collect();
    let visible_ids: HashSet<String> = visible_rows
        .iter()
        .filter(|row| row.setting.is_some())
        .map(|row| row.device.id.clone())
        .collect();
    let selected_ids = selected();
    let template_snapshot = batch_template();
    let batch_errors = batch_sensor_errors(&rows, &selected_ids, &template_snapshot);
    let draft_snapshot = drafts();
    let draft_errors = setting_errors(&rows, &draft_snapshot);
    let diffs = build_diffs(&rows, &draft_snapshot);

    let confirm_diffs = diffs.clone();
    let on_confirm = move |_| {
        let current_diffs = confirm_diffs.clone();
        async move {
            is_executing.set(true);
            let toast_api = use_toast();
            let client = Client::new();
            let endpoint = endpoint();
            let project_key = project().project_key;
            let errors = execute_diffs(&client, &endpoint, &project_key, &current_diffs).await;
            is_executing.set(false);

            if errors.is_empty() {
                toast_api.success(
                    "主動監控設定已套用".to_string(),
                    ToastOptions::new().duration(Duration::from_secs(5)),
                );
            } else {
                toast_api.error(
                    format!("套用完成，但有 {} 台裝置失敗", errors.len()),
                    ToastOptions::new()
                        .description(errors.join("；"))
                        .duration(Duration::from_secs(15)),
                );
            }
            settings.restart();
            drafts.set(HashMap::new());
            selected.set(HashSet::new());
            step.set(PageStep::Edit);
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
                BreadcrumbItem::current("主動監控總覽"),
            ],
        }
        PageHeader { title: "主動監控總覽",
            Button {
                variant: ButtonVariant::Ghost,
                disabled: is_loading || is_executing(),
                onclick: move |_| settings.restart(),
                Icon { icon: fa_solid_icons::FaRotateLeft }
                "重新載入"
            }
        }

        if is_loading {
            div { class: "flex items-center gap-2 text-slate-500 dark:text-slate-400",
                Icon { class: "animate-spin", icon: fa_solid_icons::FaSpinner }
                "正在載入主動監控設定..."
            }
        } else if let Some(error) = load_error {
            div { class: "border border-red-600 dark:border-red-400 rounded-lg p-4 text-red-600 dark:text-red-400",
                "主動監控設定載入失敗：{error}"
            }
        } else if step() == PageStep::Edit {
            div { class: "flex flex-col gap-5",
                div { class: "flex items-end gap-4 flex-wrap",
                    div { class: "flex flex-col gap-1 min-w-64",
                        Label { html_for: "active_overview_filter", "篩選裝置" }
                        DxInput {
                            id: "active_overview_filter",
                            placeholder: "輸入裝置名稱或 ID",
                            value: filter(),
                            onchange: move |event: FormEvent| filter.set(event.value()),
                        }
                    }
                    div { class: "text-sm text-slate-500 dark:text-slate-400",
                        "已選取 {selected_ids.len()} 台可用裝置"
                    }
                }

                div { class: "border border-slate-200 dark:border-zinc-800 rounded-lg p-4 flex flex-col gap-4",
                    div {
                        h2 { class: "text-lg font-bold", "批次套用完整設定" }
                        p { class: "text-sm text-slate-500 dark:text-slate-400 mt-1",
                            "先勾選裝置，再設定一組完整設定並套用。未勾選的裝置不會受到影響。"
                        }
                    }
                    div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4",
                        div { class: "flex items-center gap-2",
                            Switch {
                                checked: template_snapshot.setting.enable,
                                on_checked_change: move |value| batch_template.write().setting.enable = value,
                            }
                            span { "啟用" }
                        }
                        div { class: "flex flex-col gap-1",
                            Label { html_for: "active_batch_period", "週期" }
                            DxInput {
                                id: "active_batch_period",
                                placeholder: "例如：5m、1h、1d",
                                value: template_snapshot.setting.period.clone(),
                                onchange: move |event: FormEvent| batch_template.write().setting.period = event.value(),
                            }
                        }
                        div { class: "flex flex-col gap-1",
                            Label { html_for: "active_batch_min_uploads", "最小上傳次數" }
                            DxInput {
                                id: "active_batch_min_uploads",
                                placeholder: "不限",
                                value: template_snapshot.min_uploads_text.clone(),
                                onchange: move |event: FormEvent| {
                                    let text = event.value();
                                    let (value, valid) = parse_optional_i32(&text);
                                    let mut template = batch_template.write();
                                    template.min_uploads_text = text;
                                    template.min_uploads_valid = valid;
                                    if valid {
                                        template.setting.min_uploads = value;
                                    }
                                },
                            }
                        }
                        div { class: "flex flex-col gap-1",
                            Label { html_for: "active_batch_max_uploads", "最大上傳次數" }
                            DxInput {
                                id: "active_batch_max_uploads",
                                placeholder: "不限",
                                value: template_snapshot.max_uploads_text.clone(),
                                onchange: move |event: FormEvent| {
                                    let text = event.value();
                                    let (value, valid) = parse_optional_i32(&text);
                                    let mut template = batch_template.write();
                                    template.max_uploads_text = text;
                                    template.max_uploads_valid = valid;
                                    if valid {
                                        template.setting.max_uploads = value;
                                    }
                                },
                            }
                        }
                        div { class: "flex flex-col gap-1",
                            Label { html_for: "active_batch_sensor", "Sensor" }
                            Select::<Option<String>> {
                                default_value: template_snapshot.setting.sensor.clone(),
                                on_value_change: move |value: Option<Option<String>>| {
                                    batch_template.write().setting.sensor = value.flatten();
                                },
                                SelectGroup {
                                    SensorOptions {
                                        sensor_ids: sensor_ids(),
                                        include_all: true,
                                    }
                                }
                            }
                        }
                    }
                    if !template_snapshot.min_uploads_valid || !template_snapshot.max_uploads_valid {
                        p { class: "text-sm text-red-600 dark:text-red-400",
                            "批次模板的上傳次數必須是整數。"
                        }
                    }
                    for error in batch_errors.iter() {
                        p { class: "text-sm text-red-600 dark:text-red-400", "{error}" }
                    }
                    div { class: "flex flex-wrap items-center gap-3",
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: visible_ids.is_empty() || is_executing(),
                            onclick: move |_| selected.set(visible_ids.clone()),
                            "全選可用裝置"
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: selected_ids.is_empty() || is_executing(),
                            onclick: move |_| selected.set(HashSet::new()),
                            "取消全選"
                        }
                        Button {
                            disabled: selected_ids.is_empty()
                                                            || !template_snapshot.is_valid()
                                                            || !batch_errors.is_empty()
                                || is_executing(),
                            onclick: {
                                let rows_for_batch = rows.clone();
                                move |_| {
                                    let template = batch_template();
                                    let selected_ids = selected();
                                    if !template.is_valid()
                                        || !batch_sensor_errors(&rows_for_batch, &selected_ids, &template)
                                            .is_empty()
                                    {
                                        return;
                                    }
                                    drafts
                                        .with_mut(|draft_map| {
                                            for device_id in &selected_ids {
                                                let Some(row) = rows_for_batch
                                                    .iter()
                                                    .find(|row| row.device.id == *device_id) else {
                                                    continue;
                                                };
                                                let Some(current) = row.setting.as_ref() else {
                                                    continue;
                                                };
                                                draft_map
                                                    .insert(
                                                        device_id.clone(),
                                                        template.for_device(&row.device, current),
                                                    );
                                            }
                                        });
                                }
                            },
                            "套用到已勾選裝置"
                        }
                    }
                }

                if !draft_errors.is_empty() {
                    div { class: "border border-red-600 dark:border-red-400 rounded-lg p-3 text-sm text-red-600 dark:text-red-400",
                        for error in draft_errors.iter() {
                            p { "{error}" }
                        }
                    }
                }

                div { class: "flex flex-col border border-slate-200 dark:border-zinc-800 rounded-lg overflow-hidden",
                    div { class: "hidden lg:grid grid-cols-[minmax(14rem,1.4fr)_minmax(8rem,.7fr)_minmax(12rem,1fr)_minmax(9rem,.7fr)_minmax(9rem,.7fr)_minmax(13rem,1.1fr)] gap-3 p-3 bg-slate-50 dark:bg-zinc-900 text-sm font-semibold",
                        div { "裝置" }
                        div { "啟用" }
                        div { "週期" }
                        div { "最小上傳次數" }
                        div { "最大上傳次數" }
                        div { "Sensor" }
                    }
                    if visible_rows.is_empty() {
                        div { class: "p-4 text-sm text-slate-500 dark:text-slate-400",
                            "沒有符合的裝置"
                        }
                    }
                    for row in visible_rows {
                        if let Some(setting) = row.setting {
                            ActiveSettingRowView {
                                key: "{row.device.id}",
                                device: row.device,
                                setting,
                                drafts,
                                selected,
                            }
                        } else if let Some(error) = row.error {
                            UnavailableSettingRow {
                                key: "{row.device.id}",
                                device: row.device,
                                error,
                            }
                        }
                    }
                }

                div { class: "flex justify-end",
                    Button {
                        disabled: diffs.is_empty() || !draft_errors.is_empty() || is_executing(),
                        onclick: move |_| step.set(PageStep::Preview),
                        "預覽變更（{diffs.len()}）"
                    }
                }
            }
        } else {
            div { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-bold", "確認變更" }
                if !draft_errors.is_empty() {
                    div { class: "border border-red-600 dark:border-red-400 rounded-lg p-3 text-sm text-red-600 dark:text-red-400",
                        for error in draft_errors.iter() {
                            p { "{error}" }
                        }
                    }
                }
                if diffs.is_empty() {
                    p { class: "text-slate-500 dark:text-slate-400", "沒有偵測到任何異動" }
                }
                for (index , diff) in diffs.iter().enumerate() {
                    ActiveSettingDiffCard { key: "{index}", diff: diff.clone() }
                }
                div { class: "flex justify-end gap-4 mt-2",
                    Button {
                        variant: ButtonVariant::Secondary,
                        disabled: is_executing(),
                        onclick: move |_| step.set(PageStep::Edit),
                        "上一步"
                    }
                    Button {
                        disabled: is_executing() || diffs.is_empty() || !draft_errors.is_empty(),
                        onclick: on_confirm,
                        if is_executing() {
                            "套用中..."
                        } else {
                            "確認套用"
                        }
                    }
                }
            }
        }
        div { class: "h-48" }
    }
}

#[component]
fn SensorOptions(sensor_ids: Vec<String>, include_all: bool) -> Element {
    let start_index = if include_all { 1 } else { 0 };
    rsx! {
        if include_all {
            SelectOption::<Option<String>> { index: 0usize, value: None, text_value: "全部感測器", "全部感測器" }
        }
        for (index , sensor_id) in sensor_ids.iter().enumerate() {
            SelectOption::<Option<String>> {
                key: "{sensor_id}",
                index: index + start_index,
                value: Some(sensor_id.clone()),
                text_value: sensor_id.clone(),
                "{sensor_id}"
            }
        }
    }
}

#[component]
fn ActiveSettingRowView(
    device: Device,
    setting: ActiveDevice,
    drafts: Signal<HashMap<String, SettingDraft>>,
    selected: Signal<HashSet<String>>,
) -> Element {
    let device_id = device.id.clone();
    let current_draft = drafts()
        .get(&device_id)
        .cloned()
        .unwrap_or_else(|| SettingDraft::from_setting(&setting));
    let sensor_ids: Vec<String> = device
        .sensors
        .iter()
        .flatten()
        .map(|sensor| sensor.id.clone())
        .collect();
    let is_selected = selected().contains(&device_id);
    let selected_id = device_id.clone();
    let on_selected = move |state: CheckboxState| {
        selected.with_mut(|ids| {
            if state == CheckboxState::Checked {
                ids.insert(selected_id.clone());
            } else {
                ids.remove(&selected_id);
            }
        });
    };

    let enable_id = device_id.clone();
    let enable_setting = setting.clone();
    let on_enable = move |value: bool| {
        drafts.with_mut(|draft_map| {
            let draft = draft_map
                .entry(enable_id.clone())
                .or_insert_with(|| SettingDraft::from_setting(&enable_setting));
            draft.setting.enable = value;
        });
    };

    let period_id = device_id.clone();
    let period_setting = setting.clone();
    let on_period = move |event: FormEvent| {
        drafts.with_mut(|draft_map| {
            let draft = draft_map
                .entry(period_id.clone())
                .or_insert_with(|| SettingDraft::from_setting(&period_setting));
            draft.setting.period = event.value();
        });
    };

    let min_id = device_id.clone();
    let min_setting = setting.clone();
    let on_min_uploads = move |event: FormEvent| {
        let text = event.value();
        let (value, valid) = parse_optional_i32(&text);
        drafts.with_mut(|draft_map| {
            let draft = draft_map
                .entry(min_id.clone())
                .or_insert_with(|| SettingDraft::from_setting(&min_setting));
            draft.min_uploads_text = text;
            draft.min_uploads_valid = valid;
            if valid {
                draft.setting.min_uploads = value;
            }
        });
    };

    let max_id = device_id.clone();
    let max_setting = setting.clone();
    let on_max_uploads = move |event: FormEvent| {
        let text = event.value();
        let (value, valid) = parse_optional_i32(&text);
        drafts.with_mut(|draft_map| {
            let draft = draft_map
                .entry(max_id.clone())
                .or_insert_with(|| SettingDraft::from_setting(&max_setting));
            draft.max_uploads_text = text;
            draft.max_uploads_valid = valid;
            if valid {
                draft.setting.max_uploads = value;
            }
        });
    };

    let sensor_id = device_id.clone();
    let sensor_setting = setting.clone();
    let on_sensor = move |value: Option<Option<String>>| {
        drafts.with_mut(|draft_map| {
            let draft = draft_map
                .entry(sensor_id.clone())
                .or_insert_with(|| SettingDraft::from_setting(&sensor_setting));
            draft.setting.sensor = value.flatten();
        });
    };

    let select_key = format!(
        "{}-{}",
        device_id,
        current_draft.setting.sensor.as_deref().unwrap_or("all")
    );

    rsx! {
        div { class: "grid grid-cols-1 gap-4 p-3 border-t border-slate-200 dark:border-zinc-800 lg:grid-cols-[minmax(14rem,1.4fr)_minmax(8rem,.7fr)_minmax(12rem,1fr)_minmax(9rem,.7fr)_minmax(9rem,.7fr)_minmax(13rem,1.1fr)] lg:items-center lg:gap-3 lg:border-t-0",
            div { class: "flex items-start gap-3 min-w-0",
                Checkbox {
                    checked: if is_selected { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: on_selected,
                }
                div { class: "min-w-0",
                    p { class: "font-semibold break-words", "{device.name}" }
                    p { class: "text-xs text-slate-500 dark:text-slate-400 break-all",
                        "{device.id}"
                    }
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "lg:hidden text-sm text-slate-500 dark:text-slate-400 w-32 shrink-0",
                    "啟用"
                }
                Switch {
                    checked: current_draft.setting.enable,
                    on_checked_change: on_enable,
                }
                span { class: "text-sm",
                    if current_draft.setting.enable {
                        "啟用"
                    } else {
                        "停用"
                    }
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "lg:hidden text-sm text-slate-500 dark:text-slate-400 w-32 shrink-0",
                    "週期"
                }
                DxInput {
                    class: "w-full",
                    placeholder: "例如：5m、1h、1d",
                    value: current_draft.setting.period.clone(),
                    onchange: on_period,
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "lg:hidden text-sm text-slate-500 dark:text-slate-400 w-32 shrink-0",
                    "最小上傳次數"
                }
                div { class: "w-full",
                    DxInput {
                        class: "w-full",
                        placeholder: "不限",
                        value: current_draft.min_uploads_text.clone(),
                        onchange: on_min_uploads,
                    }
                    if !current_draft.min_uploads_valid {
                        p { class: "text-xs text-red-600 dark:text-red-400 mt-1",
                            "必須是整數"
                        }
                    }
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "lg:hidden text-sm text-slate-500 dark:text-slate-400 w-32 shrink-0",
                    "最大上傳次數"
                }
                div { class: "w-full",
                    DxInput {
                        class: "w-full",
                        placeholder: "不限",
                        value: current_draft.max_uploads_text.clone(),
                        onchange: on_max_uploads,
                    }
                    if !current_draft.max_uploads_valid {
                        p { class: "text-xs text-red-600 dark:text-red-400 mt-1",
                            "必須是整數"
                        }
                    }
                }
            }
            div { class: "flex items-center gap-2 min-w-0",
                span { class: "lg:hidden text-sm text-slate-500 dark:text-slate-400 w-32 shrink-0",
                    "Sensor"
                }
                Select::<Option<String>> {
                    key: "{select_key}",
                    class: "w-full",
                    default_value: current_draft.setting.sensor.clone(),
                    on_value_change: on_sensor,
                    SelectGroup {
                        SensorOptions { sensor_ids, include_all: true }
                    }
                }
            }
        }
    }
}

#[component]
fn UnavailableSettingRow(device: Device, error: String) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-2 p-3 border-t border-slate-200 dark:border-zinc-800 lg:grid-cols-[minmax(14rem,1.4fr)_1fr] lg:items-center lg:gap-3",
            div {
                p { class: "font-semibold break-words", "{device.name}" }
                p { class: "text-xs text-slate-500 dark:text-slate-400 break-all",
                    "{device.id}"
                }
            }
            p { class: "text-sm text-red-600 dark:text-red-400",
                "無法載入設定：{error}（此裝置會略過批次套用）"
            }
        }
    }
}

#[component]
fn ActiveSettingDiffCard(diff: ActiveSettingDiff) -> Element {
    rsx! {
        div { class: "border border-slate-200 dark:border-zinc-800 rounded-lg p-3",
            div { class: "flex items-center gap-2 flex-wrap mb-3",
                span { class: "px-2 py-0.5 border border-blue-600 dark:border-blue-400 rounded text-xs text-blue-600 dark:text-blue-400",
                    "更新"
                }
                span { class: "font-semibold", "{diff.device_name}" }
                span { class: "text-xs text-slate-500 dark:text-slate-400", "ID: {diff.device_id}" }
            }
            div { class: "flex flex-col gap-1",
                for field in diff.fields.iter() {
                    div { class: "grid grid-cols-[minmax(8rem,auto)_1fr] gap-3 text-sm",
                        span { class: "text-slate-500 dark:text-slate-400", "{field.label}" }
                        span { "{field.before} → {field.after}" }
                    }
                }
            }
        }
    }
}
