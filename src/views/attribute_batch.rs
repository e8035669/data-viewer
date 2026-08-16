//! Key-focused batch editor for `Device`/`Sensor` attributes.
//!
//! Lets an operator pick one attribute `key` and, across many devices/sensors at once,
//! bulk-set a shared value (with select-all + exclude), edit each value individually, or
//! bulk-delete the key — then review a diff before applying.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use dioxus::prelude::*;
use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;

use crate::{
    api::ApiHelper,
    components::{
        button::{Button, ButtonVariant},
        checkbox::Checkbox,
        label::Label,
        radio_group::{RadioGroup, RadioItem},
        select::{Select, SelectGroup, SelectOption},
    },
    models::{Attribute, Device, EditDevice, EditSensor, Endpoint},
    ui::{
        breadcrumb::{Breadcrumb, BreadcrumbItem},
        custom::DxInput,
        page_header::PageHeader,
    },
    views::{global::HeaderContext, sensor_v2::ProjectContext},
    Route,
};

// ─── Scope (which set of owners the key search covers) ──────────────────────

#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) enum AttrScope {
    #[default]
    AllDevice,
    OneDeviceSensors,
    AllDevicesSensors,
}

impl AttrScope {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            AttrScope::AllDevice => "all_device",
            AttrScope::OneDeviceSensors => "one_device_sensor",
            AttrScope::AllDevicesSensors => "all_device_sensor",
        }
    }

    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "one_device_sensor" => AttrScope::OneDeviceSensors,
            "all_device_sensor" => AttrScope::AllDevicesSensors,
            _ => AttrScope::AllDevice,
        }
    }
}

// ─── Owner identity (a Device or a Sensor under a Device) ────────────────────

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum OwnerKey {
    Device(String),
    Sensor(String, String),
}

/// One occurrence of the searched key for an owner, or a placeholder row when the owner
/// has no entry for that key yet (`index: None`). Duplicate keys on the same owner each
/// get their own row, distinguished by `index` and `occurrence`.
#[derive(Clone, PartialEq)]
struct AttrRow {
    owner: OwnerKey,
    owner_label: String,
    index: Option<usize>,
    value: String,
    /// (this occurrence, total occurrences) — only set when the owner has the key more than once.
    occurrence: Option<(usize, usize)>,
}

fn row_id(row: &AttrRow) -> String {
    let owner_part = match &row.owner {
        OwnerKey::Device(d) => format!("d:{d}"),
        OwnerKey::Sensor(d, s) => format!("s:{d}:{s}"),
    };
    match row.index {
        Some(i) => format!("{owner_part}#{i}"),
        None => format!("{owner_part}#new"),
    }
}

fn push_owner_rows(rows: &mut Vec<AttrRow>, owner: OwnerKey, owner_label: String, attrs: &[Attribute], key: &str) {
    let matches: Vec<(usize, &Attribute)> = attrs.iter().enumerate().filter(|(_, a)| a.key == key).collect();
    if matches.is_empty() {
        rows.push(AttrRow {
            owner,
            owner_label,
            index: None,
            value: String::new(),
            occurrence: None,
        });
        return;
    }
    let total = matches.len();
    for (n, (i, a)) in matches.into_iter().enumerate() {
        rows.push(AttrRow {
            owner: owner.clone(),
            owner_label: owner_label.clone(),
            index: Some(i),
            value: a.value.clone(),
            occurrence: if total > 1 { Some((n + 1, total)) } else { None },
        });
    }
}

fn build_rows(devices: &[Device], scope: AttrScope, key: &str, selected_device_id: &str) -> Vec<AttrRow> {
    let mut rows = Vec::new();
    let key = key.trim();
    if key.is_empty() {
        return rows;
    }

    match scope {
        AttrScope::AllDevice => {
            for d in devices {
                let label = format!("{} (ID: {})", d.name, d.id);
                push_owner_rows(
                    &mut rows,
                    OwnerKey::Device(d.id.clone()),
                    label,
                    d.attributes.as_deref().unwrap_or(&[]),
                    key,
                );
            }
        }
        AttrScope::OneDeviceSensors => {
            if let Some(d) = devices.iter().find(|d| d.id == selected_device_id) {
                for s in d.sensors.iter().flatten() {
                    let label = format!("{} / {} (ID: {})", d.name, s.name, s.id);
                    push_owner_rows(
                        &mut rows,
                        OwnerKey::Sensor(d.id.clone(), s.id.clone()),
                        label,
                        s.attributes.as_deref().unwrap_or(&[]),
                        key,
                    );
                }
            }
        }
        AttrScope::AllDevicesSensors => {
            for d in devices {
                for s in d.sensors.iter().flatten() {
                    let label = format!("{} / {} (ID: {})", d.name, s.name, s.id);
                    push_owner_rows(
                        &mut rows,
                        OwnerKey::Sensor(d.id.clone(), s.id.clone()),
                        label,
                        s.attributes.as_deref().unwrap_or(&[]),
                        key,
                    );
                }
            }
        }
    }

    rows
}

// ─── Staged edits ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub(crate) enum PendingOp {
    SetValue(String),
    Delete,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ChangeKind {
    Create,
    Update,
    Delete,
}

impl ChangeKind {
    pub(crate) fn label_and_class(&self) -> (&'static str, &'static str) {
        match self {
            ChangeKind::Create => (
                "新增",
                "text-green-600 dark:text-green-400 border-green-600 dark:border-green-400",
            ),
            ChangeKind::Update => (
                "修改",
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
struct KeyDiff {
    owner: OwnerKey,
    owner_label: String,
    kind: ChangeKind,
    index: Option<usize>,
    before: Option<String>,
    after: Option<String>,
}

fn build_diffs(rows: &[AttrRow], pending: &HashMap<String, PendingOp>) -> Vec<KeyDiff> {
    let mut diffs = Vec::new();
    for row in rows {
        let Some(op) = pending.get(&row_id(row)) else {
            continue;
        };
        match (row.index, op) {
            (Some(_), PendingOp::SetValue(v)) => {
                if *v != row.value {
                    diffs.push(KeyDiff {
                        owner: row.owner.clone(),
                        owner_label: row.owner_label.clone(),
                        kind: ChangeKind::Update,
                        index: row.index,
                        before: Some(row.value.clone()),
                        after: Some(v.clone()),
                    });
                }
            }
            (None, PendingOp::SetValue(v)) => {
                diffs.push(KeyDiff {
                    owner: row.owner.clone(),
                    owner_label: row.owner_label.clone(),
                    kind: ChangeKind::Create,
                    index: None,
                    before: None,
                    after: Some(v.clone()),
                });
            }
            (Some(_), PendingOp::Delete) => {
                diffs.push(KeyDiff {
                    owner: row.owner.clone(),
                    owner_label: row.owner_label.clone(),
                    kind: ChangeKind::Delete,
                    index: row.index,
                    before: Some(row.value.clone()),
                    after: None,
                });
            }
            (None, PendingOp::Delete) => {}
        }
    }
    diffs
}

/// Rebuilds one owner's full attribute list by applying its staged create/update/delete ops
/// for `key`. Deletes are resolved by index before creates are appended.
fn apply_attr_changes(attrs: Vec<Attribute>, changes: &[&KeyDiff], key: &str) -> Vec<Attribute> {
    let mut delete_set: HashSet<usize> = HashSet::new();
    let mut update_map: HashMap<usize, String> = HashMap::new();
    let mut creates: Vec<String> = Vec::new();
    for c in changes {
        match (c.index, &c.after) {
            (Some(i), Some(v)) => {
                update_map.insert(i, v.clone());
            }
            (Some(i), None) => {
                delete_set.insert(i);
            }
            (None, Some(v)) => creates.push(v.clone()),
            (None, None) => {}
        }
    }

    let mut new_attrs: Vec<Attribute> = attrs
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !delete_set.contains(i))
        .map(|(i, mut a)| {
            if let Some(v) = update_map.get(&i) {
                a.value = v.clone();
            }
            a
        })
        .collect();
    for v in creates {
        new_attrs.push(Attribute {
            key: key.to_string(),
            value: v,
        });
    }
    new_attrs
}

/// Applies every staged diff by grouping them per owner and issuing one `update_device`/
/// `update_sensor` call per owner. Returns human-readable errors for any failed owner;
/// the rest still get applied so a partial failure doesn't block unrelated changes.
async fn execute_key_diffs(
    client: &Client,
    endpoint: &Endpoint,
    project_key: &str,
    devices: &[Device],
    key: &str,
    diffs: &[KeyDiff],
) -> Vec<String> {
    let mut grouped: HashMap<OwnerKey, Vec<&KeyDiff>> = HashMap::new();
    for d in diffs {
        grouped.entry(d.owner.clone()).or_default().push(d);
    }

    let mut errors = Vec::new();
    for (owner, changes) in grouped {
        match owner {
            OwnerKey::Device(device_id) => {
                let Some(device) = devices.iter().find(|d| d.id == device_id) else {
                    errors.push(format!("找不到裝置 {device_id}"));
                    continue;
                };
                let attrs = apply_attr_changes(device.attributes.clone().unwrap_or_default(), &changes, key);
                let edit = EditDevice {
                    name: device.name.clone(),
                    kind: device.kind.clone(),
                    attributes: Some(attrs),
                    ..Default::default()
                };
                if let Err(e) = ApiHelper::update_device(client, endpoint, project_key, &device_id, &edit).await {
                    errors.push(format!("裝置「{}」更新失敗: {e}", device.name));
                }
            }
            OwnerKey::Sensor(device_id, sensor_id) => {
                let Some(device) = devices.iter().find(|d| d.id == device_id) else {
                    errors.push(format!("找不到裝置 {device_id}"));
                    continue;
                };
                let Some(sensor) = device.sensors.iter().flatten().find(|s| s.id == sensor_id) else {
                    errors.push(format!("找不到感測器 {sensor_id}"));
                    continue;
                };
                let attrs = apply_attr_changes(sensor.attributes.clone().unwrap_or_default(), &changes, key);
                let edit = EditSensor {
                    name: sensor.name.clone(),
                    kind: sensor.kind,
                    attributes: Some(attrs),
                    ..Default::default()
                };
                if let Err(e) =
                    ApiHelper::update_sensor(client, endpoint, project_key, &device_id, &sensor_id, &edit).await
                {
                    errors.push(format!("感測器「{}」更新失敗: {e}", sensor.name));
                }
            }
        }
    }
    errors
}

// ─── Page ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default)]
enum PageStep {
    #[default]
    Edit,
    Preview,
}

#[component]
pub fn ProjectAttributeBatch(project_name: ReadSignal<String>) -> Element {
    use_effect(move || {
        consume_context::<HeaderContext>().set_title("批次修改屬性");
    });

    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;
    let mut project_resource: Resource<Result<Vec<Device>>> = use_context();

    let mut scope = use_signal(AttrScope::default);
    let mut selected_device_id =
        use_signal(move || project_meta().first().map(|d| d.id.clone()).unwrap_or_default());
    let mut key = use_signal(String::new);
    let mut pending: Signal<HashMap<String, PendingOp>> = use_signal(HashMap::new);
    let mut selected: Signal<HashSet<String>> = use_signal(HashSet::new);
    let mut bulk_value = use_signal(String::new);
    let mut step = use_signal(PageStep::default);
    let mut is_executing = use_signal(|| false);

    let rows = use_memo(move || build_rows(&project_meta(), scope(), &key(), &selected_device_id()));

    // The row set (and therefore every staged row id) changes whenever scope/key/device
    // selection changes, so any previously staged edits or selections are no longer valid.
    use_effect(move || {
        let _ = (scope(), key(), selected_device_id());
        pending.set(HashMap::new());
        selected.set(HashSet::new());
    });

    let diffs = use_memo(move || build_diffs(&rows(), &pending()));

    let on_confirm = move |_| async move {
        is_executing.set(true);
        let toast_api = use_toast();
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let devices = project_meta();
        let key_val = key();
        let current_diffs = diffs();
        let errors = execute_key_diffs(&client, &ep, &pk, &devices, &key_val, &current_diffs).await;
        is_executing.set(false);

        if errors.is_empty() {
            toast_api.success(
                "已套用變更".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
        } else {
            toast_api.error(
                format!("套用完成，但有 {} 項失敗", errors.len()),
                ToastOptions::new()
                    .description(errors.join("; "))
                    .duration(Duration::from_secs(15)),
            );
        }
        project_resource.restart();
        pending.set(HashMap::new());
        selected.set(HashSet::new());
        step.set(PageStep::Edit);
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
                BreadcrumbItem::current("批次修改屬性"),
            ],
        }
        PageHeader { title: "批次修改屬性" }

        if step() == PageStep::Edit {
            div { class: "flex flex-col gap-6",
                div {
                    h2 { class: "text-lg font-bold mb-2", "1. 選擇範圍" }
                    RadioGroup {
                        horizontal: true,
                        value: scope().as_str().to_string(),
                        on_value_change: move |v: String| scope.set(AttrScope::from_str(&v)),
                        RadioItem { index: 0usize, value: "all_device".to_string(),
                            "所有 Device 的屬性"
                        }
                        RadioItem {
                            index: 1usize,
                            value: "one_device_sensor".to_string(),
                            "單一 Device 底下所有 Sensor 的屬性"
                        }
                        RadioItem {
                            index: 2usize,
                            value: "all_device_sensor".to_string(),
                            "所有 Device 底下所有 Sensor 的屬性"
                        }
                    }
                }

                if scope() == AttrScope::OneDeviceSensors {
                    div {
                        Label { html_for: "attr_batch_device", "選擇 Device" }
                        Select::<String> {
                            default_value: Some(selected_device_id()),
                            on_value_change: move |v: Option<String>| {
                                if let Some(v) = v {
                                    selected_device_id.set(v);
                                }
                            },
                            SelectGroup {
                                for (i , d) in project_meta().into_iter().enumerate() {
                                    SelectOption::<String> {
                                        key: "{d.id}",
                                        index: i,
                                        value: d.id.clone(),
                                        text_value: d.name.clone(),
                                        "{d.name}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    h2 { class: "text-lg font-bold mb-2", "2. 輸入 Attribute Key" }
                    DxInput {
                        placeholder: "attribute key",
                        value: key(),
                        onchange: move |e: FormEvent| key.set(e.value()),
                    }
                }

                if !key().trim().is_empty() {
                    div { class: "flex flex-col gap-3",
                        h2 { class: "text-lg font-bold", "3. 修改" }
                        div { class: "flex items-center gap-2 flex-wrap",
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| selected.set(rows().iter().map(row_id).collect()),
                                "全選"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| selected.write().clear(),
                                "取消全選"
                            }
                            DxInput {
                                class: "flex-1 min-w-48",
                                placeholder: "統一輸入的值",
                                value: bulk_value(),
                                onchange: move |e: FormEvent| bulk_value.set(e.value()),
                            }
                            Button {
                                disabled: selected().is_empty(),
                                onclick: move |_| {
                                    let value = bulk_value();
                                    let ids = selected();
                                    pending
                                        .with_mut(|p| {
                                            for rid in ids {
                                                p.insert(rid, PendingOp::SetValue(value.clone()));
                                            }
                                        });
                                },
                                "套用到已勾選"
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                disabled: selected().is_empty(),
                                onclick: move |_| {
                                    let ids = selected();
                                    let deletable: HashSet<String> = rows()
                                        .iter()
                                        .filter(|r| r.index.is_some())
                                        .map(row_id)
                                        .collect();
                                    pending
                                        .with_mut(|p| {
                                            for rid in ids {
                                                if deletable.contains(&rid) {
                                                    p.insert(rid, PendingOp::Delete);
                                                }
                                            }
                                        });
                                },
                                "刪除已勾選"
                            }
                        }

                        div { class: "flex flex-col border border-slate-200 dark:border-zinc-800 rounded-lg divide-y divide-slate-200 dark:divide-zinc-800",
                            if rows().is_empty() {
                                div { class: "p-4 text-sm text-slate-500 dark:text-slate-400",
                                    "沒有符合的裝置/感測器"
                                }
                            }
                            for row in rows() {
                                AttrRowView {
                                    key: "{row_id(&row)}",
                                    row: row.clone(),
                                    pending,
                                    selected,
                                }
                            }
                        }

                        div { class: "flex justify-end",
                            Button {
                                disabled: diffs().is_empty(),
                                onclick: move |_| step.set(PageStep::Preview),
                                "預覽變更（{diffs().len()}）"
                            }
                        }
                    }
                }
            }
        } else {
            div { class: "flex flex-col gap-3",
                h2 { class: "text-lg font-bold", "確認變更" }
                if diffs().is_empty() {
                    p { "沒有偵測到任何異動" }
                }
                for (i , d) in diffs().iter().enumerate() {
                    KeyDiffCard { key: "{i}", diff: d.clone() }
                }
                div { class: "flex justify-end gap-4 mt-4",
                    Button {
                        variant: ButtonVariant::Secondary,
                        disabled: is_executing(),
                        onclick: move |_| step.set(PageStep::Edit),
                        "上一步"
                    }
                    Button {
                        disabled: is_executing() || diffs().is_empty(),
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
fn AttrRowView(row: AttrRow, pending: Signal<HashMap<String, PendingOp>>, selected: Signal<HashSet<String>>) -> Element {
    let rid = row_id(&row);

    let is_checked = selected().contains(&rid);
    let on_toggle = {
        let rid = rid.clone();
        move |v: CheckboxState| {
            let rid = rid.clone();
            selected.with_mut(|sel| {
                if v == CheckboxState::Checked {
                    sel.insert(rid);
                } else {
                    sel.remove(&rid);
                }
            });
        }
    };

    // Plain (non-memoized) on purpose: `row.value` is an owned prop with no Signal to track, so a
    // `use_memo` here would only re-run when `pending` changes and would never pick up a later
    // refresh of `row.value` (e.g. after `project_resource.restart()` resolves) once `pending` had
    // already settled back to "no staged edit".
    let pending_op = pending().get(&rid).cloned();
    let display_value = match &pending_op {
        Some(PendingOp::SetValue(v)) => v.clone(),
        _ => row.value.clone(),
    };
    let is_delete_pending = matches!(pending_op, Some(PendingOp::Delete));

    let on_value_change = {
        let rid = rid.clone();
        move |e: FormEvent| {
            pending.write().insert(rid.clone(), PendingOp::SetValue(e.value()));
        }
    };

    let exists = row.index.is_some();
    let occurrence_badge = row.occurrence.map(|(n, total)| format!("第 {n}/{total} 筆"));

    rsx! {
        div { class: "grid grid-cols-[auto_1fr_2fr] items-center gap-3 p-3",
            Checkbox {
                checked: if is_checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: on_toggle,
            }
            div { class: "flex flex-col gap-1",
                span { class: "text-sm", "{row.owner_label}" }
                div { class: "flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400",
                    if !exists {
                        span { class: "px-1.5 py-0.5 border rounded", "不存在" }
                    }
                    if let Some(badge) = &occurrence_badge {
                        span { "{badge}" }
                    }
                    if is_delete_pending {
                        span { class: "text-red-600 dark:text-red-400", "將刪除" }
                    }
                }
            }
            DxInput {
                placeholder: if exists { "" } else { "輸入以新增此屬性" },
                value: display_value,
                onchange: on_value_change,
            }
        }
    }
}

#[component]
fn KeyDiffCard(diff: KeyDiff) -> Element {
    let (label, class) = diff.kind.label_and_class();

    rsx! {
        div { class: "border border-slate-200 dark:border-zinc-800 rounded-lg p-3 flex items-center gap-3 flex-wrap",
            span { class: "px-1.5 py-0.5 border rounded text-xs shrink-0 {class}", "{label}" }
            span { class: "text-sm", "{diff.owner_label}" }
            match (&diff.before, &diff.after) {
                (Some(b), Some(a)) => rsx! {
                    span { class: "text-sm text-slate-500 dark:text-slate-400", "「{b}」→「{a}」" }
                },
                (None, Some(a)) => rsx! {
                    span { class: "text-sm text-slate-500 dark:text-slate-400", "新值：「{a}」" }
                },
                (Some(b), None) => rsx! {
                    span { class: "text-sm text-slate-500 dark:text-slate-400", "原值：「{b}」" }
                },
                (None, None) => rsx! {},
            }
        }
    }
}
