//! Unified attribute overview: a spreadsheet-style matrix (owner × key) across many
//! devices/sensors at once, with optional baseline-diff highlighting and inline edit.
//!
//! Complements the single-key workflow in `attribute_batch.rs` — that page is for "I know the
//! key, change it everywhere"; this page is for "what does everything currently look like, and
//! which owner drifted from the others".

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;

use crate::{
    api::ApiHelper,
    components::{
        button::{Button, ButtonVariant},
        label::Label,
        radio_group::{RadioGroup, RadioItem},
        select::{Select, SelectGroup, SelectOption},
        switch::Switch,
    },
    models::{Attribute, Device, EditDevice, EditSensor, Endpoint},
    ui::{
        breadcrumb::{Breadcrumb, BreadcrumbItem},
        custom::DxInput,
        page_header::PageHeader,
    },
    views::{
        attribute_batch::{AttrScope, ChangeKind, OwnerKey, PendingOp},
        global::HeaderContext,
        sensor_v2::ProjectContext,
    },
    Route,
};

// ─── Owner data (a Device or a Sensor under a Device, with its raw attributes) ──────────────

#[derive(Clone, PartialEq)]
struct OwnerData {
    owner: OwnerKey,
    label: String,
    attrs: Vec<Attribute>,
}

fn owner_id_str(owner: &OwnerKey) -> String {
    match owner {
        OwnerKey::Device(d) => format!("d:{d}"),
        OwnerKey::Sensor(d, s) => format!("s:{d}:{s}"),
    }
}

fn collect_owners(devices: &[Device], scope: AttrScope, selected_device_id: &str) -> Vec<OwnerData> {
    let mut owners = Vec::new();
    match scope {
        AttrScope::AllDevice => {
            for d in devices {
                owners.push(OwnerData {
                    owner: OwnerKey::Device(d.id.clone()),
                    label: format!("{} (ID: {})", d.name, d.id),
                    attrs: d.attributes.clone().unwrap_or_default(),
                });
            }
        }
        AttrScope::OneDeviceSensors => {
            if let Some(d) = devices.iter().find(|d| d.id == selected_device_id) {
                for s in d.sensors.iter().flatten() {
                    owners.push(OwnerData {
                        owner: OwnerKey::Sensor(d.id.clone(), s.id.clone()),
                        label: format!("{} / {} (ID: {})", d.name, s.name, s.id),
                        attrs: s.attributes.clone().unwrap_or_default(),
                    });
                }
            }
        }
        AttrScope::AllDevicesSensors => {
            for d in devices {
                for s in d.sensors.iter().flatten() {
                    owners.push(OwnerData {
                        owner: OwnerKey::Sensor(d.id.clone(), s.id.clone()),
                        label: format!("{} / {} (ID: {})", d.name, s.name, s.id),
                        attrs: s.attributes.clone().unwrap_or_default(),
                    });
                }
            }
        }
    }
    owners
}

/// Distinct attribute keys across every owner in scope, in first-seen-then-sorted order.
fn collect_keys(owners: &[OwnerData]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut keys = Vec::new();
    for o in owners {
        for a in &o.attrs {
            if seen.insert(a.key.clone()) {
                keys.push(a.key.clone());
            }
        }
    }
    keys.sort();
    keys
}

// ─── Staged edits + diff (multi-key, unlike attribute_batch.rs's single-key diff) ───────────

#[derive(Clone, PartialEq)]
struct CellDiff {
    owner: OwnerKey,
    owner_label: String,
    attr_key: String,
    kind: ChangeKind,
    index: Option<usize>,
    before: Option<String>,
    after: Option<String>,
}

fn build_diffs(owners: &[OwnerData], pending: &HashMap<(String, String), PendingOp>) -> Vec<CellDiff> {
    let by_id: HashMap<String, &OwnerData> = owners.iter().map(|o| (owner_id_str(&o.owner), o)).collect();
    let mut diffs = Vec::new();
    for ((oid, key), op) in pending {
        let Some(o) = by_id.get(oid) else {
            continue;
        };
        let existing = o.attrs.iter().enumerate().find(|(_, a)| &a.key == key);
        match (existing, op) {
            (Some((idx, a)), PendingOp::SetValue(v)) => {
                if v != &a.value {
                    diffs.push(CellDiff {
                        owner: o.owner.clone(),
                        owner_label: o.label.clone(),
                        attr_key: key.clone(),
                        kind: ChangeKind::Update,
                        index: Some(idx),
                        before: Some(a.value.clone()),
                        after: Some(v.clone()),
                    });
                }
            }
            (None, PendingOp::SetValue(v)) => {
                diffs.push(CellDiff {
                    owner: o.owner.clone(),
                    owner_label: o.label.clone(),
                    attr_key: key.clone(),
                    kind: ChangeKind::Create,
                    index: None,
                    before: None,
                    after: Some(v.clone()),
                });
            }
            (Some((idx, a)), PendingOp::Delete) => {
                diffs.push(CellDiff {
                    owner: o.owner.clone(),
                    owner_label: o.label.clone(),
                    attr_key: key.clone(),
                    kind: ChangeKind::Delete,
                    index: Some(idx),
                    before: Some(a.value.clone()),
                    after: None,
                });
            }
            (None, PendingOp::Delete) => {}
        }
    }
    diffs
}

/// Rebuilds one owner's full attribute list by applying its staged changes, which (unlike
/// `attribute_batch.rs::apply_attr_changes`) may span several different keys at once.
fn apply_owner_changes(attrs: Vec<Attribute>, changes: &[&CellDiff]) -> Vec<Attribute> {
    let mut delete_set: HashSet<usize> = HashSet::new();
    let mut update_map: HashMap<usize, String> = HashMap::new();
    let mut creates: Vec<Attribute> = Vec::new();
    for c in changes {
        match (c.index, &c.after) {
            (Some(i), Some(v)) => {
                update_map.insert(i, v.clone());
            }
            (Some(i), None) => {
                delete_set.insert(i);
            }
            (None, Some(v)) => creates.push(Attribute {
                key: c.attr_key.clone(),
                value: v.clone(),
            }),
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
    new_attrs.extend(creates);
    new_attrs
}

/// Applies every staged diff by grouping per owner and issuing one `update_device`/
/// `update_sensor` call per owner. Returns human-readable errors for any failed owner; the
/// rest still get applied so a partial failure doesn't block unrelated changes.
async fn execute_diffs(
    client: &Client,
    endpoint: &Endpoint,
    project_key: &str,
    devices: &[Device],
    diffs: &[CellDiff],
) -> Vec<String> {
    let mut grouped: HashMap<OwnerKey, Vec<&CellDiff>> = HashMap::new();
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
                let attrs = apply_owner_changes(device.attributes.clone().unwrap_or_default(), &changes);
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
                let attrs = apply_owner_changes(sensor.attributes.clone().unwrap_or_default(), &changes);
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
pub fn ProjectAttributeOverview(project_name: ReadSignal<String>) -> Element {
    use_effect(move || {
        consume_context::<HeaderContext>().set_title("屬性總覽");
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
    let mut key_filter = use_signal(String::new);
    let mut owner_filter = use_signal(String::new);
    let mut compare_enabled = use_signal(|| false);
    let mut baseline_owner_id = use_signal(String::new);
    let mut pending: Signal<HashMap<(String, String), PendingOp>> = use_signal(HashMap::new);
    let mut step = use_signal(PageStep::default);
    let mut is_executing = use_signal(|| false);

    let owners = use_memo(move || collect_owners(&project_meta(), scope(), &selected_device_id()));
    let all_keys = use_memo(move || collect_keys(&owners()));

    let filtered_keys = use_memo(move || {
        let f = key_filter().trim().to_lowercase();
        all_keys()
            .into_iter()
            .filter(|k| f.is_empty() || k.to_lowercase().contains(&f))
            .collect::<Vec<_>>()
    });
    let filtered_owners = use_memo(move || {
        let f = owner_filter().trim().to_lowercase();
        owners()
            .into_iter()
            .filter(|o| f.is_empty() || o.label.to_lowercase().contains(&f))
            .collect()
    });

    // The owner set changes whenever scope/device selection changes, so any staged edits no
    // longer line up with the (possibly different) rows/keys — drop them.
    use_effect(move || {
        let _ = (scope(), selected_device_id());
        pending.set(HashMap::new());
    });
    // Keep the baseline pointed at a real owner whenever the owner set changes.
    use_effect(move || {
        let owns = owners();
        if !owns.iter().any(|o| owner_id_str(&o.owner) == baseline_owner_id()) {
            baseline_owner_id.set(owns.first().map(|o| owner_id_str(&o.owner)).unwrap_or_default());
        }
    });

    let diffs = use_memo(move || build_diffs(&owners(), &pending()));

    let on_confirm = move |_| async move {
        is_executing.set(true);
        let toast_api = use_toast();
        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let devices = project_meta();
        let current_diffs = diffs();
        let errors = execute_diffs(&client, &ep, &pk, &devices, &current_diffs).await;
        is_executing.set(false);

        if errors.is_empty() {
            toast_api.success("已套用變更".to_string(), ToastOptions::new().duration(Duration::from_secs(5)));
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
                BreadcrumbItem::current("屬性總覽"),
            ],
        }
        PageHeader { title: "屬性總覽" }

        if step() == PageStep::Edit {
            div { class: "flex flex-col gap-4",
                div {
                    h2 { class: "text-lg font-bold mb-2", "範圍" }
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
                        Label { html_for: "attr_overview_device", "選擇 Device" }
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

                div { class: "flex items-end gap-4 flex-wrap",
                    div { class: "flex flex-col gap-1",
                        Label { html_for: "attr_overview_key_filter", "篩選 Key" }
                        DxInput {
                            placeholder: "輸入關鍵字篩選欄位",
                            value: key_filter(),
                            onchange: move |e: FormEvent| key_filter.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        Label { html_for: "attr_overview_owner_filter", "篩選 裝置/感測器" }
                        DxInput {
                            placeholder: "輸入名稱或 ID 篩選列",
                            value: owner_filter(),
                            onchange: move |e: FormEvent| owner_filter.set(e.value()),
                        }
                    }
                    div { class: "flex items-center gap-2",
                        Switch {
                            checked: compare_enabled(),
                            on_checked_change: move |b| compare_enabled.set(b),
                        }
                        span { class: "text-sm", "與基準比對（標示差異）" }
                    }
                    if compare_enabled() {
                        div { class: "flex flex-col gap-1",
                            Label { html_for: "attr_overview_baseline", "基準" }
                            Select::<String> {
                                default_value: Some(baseline_owner_id()),
                                on_value_change: move |v: Option<String>| {
                                    if let Some(v) = v {
                                        baseline_owner_id.set(v);
                                    }
                                },
                                SelectGroup {
                                    for o in owners() {
                                        SelectOption::<String> {
                                            key: "{owner_id_str(&o.owner)}",
                                            index: 0usize,
                                            value: owner_id_str(&o.owner),
                                            text_value: o.label.clone(),
                                            "{o.label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "hidden lg:block",
                    AttributeMatrix {
                        owners: filtered_owners(),
                        keys: filtered_keys(),
                        pending,
                        baseline_owner_id: baseline_owner_id(),
                        compare_enabled: compare_enabled(),
                    }
                }
                div { class: "lg:hidden",
                    OwnerCardsMobile {
                        owners: filtered_owners(),
                        keys: filtered_keys(),
                        pending,
                        baseline_owner_id: baseline_owner_id(),
                        compare_enabled: compare_enabled(),
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
        } else {
            div { class: "flex flex-col gap-3",
                h2 { class: "text-lg font-bold", "確認變更" }
                if diffs().is_empty() {
                    p { "沒有偵測到任何異動" }
                }
                for (i , d) in diffs().iter().enumerate() {
                    CellDiffCard { key: "{i}", diff: d.clone() }
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

// ─── Matrix table (desktop/wide screens) ─────────────────────────────────────

/// Shared by the desktop table and the mobile card list so both agree on what "the baseline
/// owner's attributes" means.
fn build_baseline_map(
    owners: &[OwnerData],
    baseline_owner_id: &str,
    compare_enabled: bool,
) -> Option<HashMap<String, String>> {
    if !compare_enabled {
        return None;
    }
    owners
        .iter()
        .find(|o| owner_id_str(&o.owner) == baseline_owner_id)
        .map(|o| o.attrs.iter().map(|a| (a.key.clone(), a.value.clone())).collect())
}

/// Shared plumbing for a single editable (owner, key) cell — "current value accounting for any
/// staged edit" and "has a staged edit" — used by both the desktop table cell and the mobile
/// card row.
///
/// Plain (non-memoized) on purpose: `original` is an owned value with no Signal to track, so a
/// `use_memo` here would only re-evaluate when `pending` changes and would never pick up a later
/// refresh of `original` (e.g. after `project_resource.restart()` resolves with new server data)
/// once `pending` had already settled back to "no staged edit". Recomputing on every render keeps
/// it in sync with fresh `original` values.
fn cell_state(
    cell_id: &(String, String),
    original: &Option<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
) -> (Option<String>, bool) {
    let pending_op = pending().get(cell_id).cloned();
    let is_dirty = pending_op.is_some();
    let display_value = match pending_op {
        Some(PendingOp::SetValue(v)) => Some(v),
        Some(PendingOp::Delete) => None,
        None => original.clone(),
    };
    (display_value, is_dirty)
}

#[component]
fn AttributeMatrix(
    owners: Vec<OwnerData>,
    keys: Vec<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_owner_id: String,
    compare_enabled: bool,
) -> Element {
    let baseline_map = build_baseline_map(&owners, &baseline_owner_id, compare_enabled);
    let colspan = (keys.len() + 1).to_string();

    rsx! {
        div { class: "overflow-auto max-h-[65lvh] border border-slate-200 dark:border-zinc-800 rounded-lg",
            table { class: "border-separate border-spacing-0 text-sm w-full",
                thead {
                    tr { class: "bg-slate-50 dark:bg-zinc-900",
                        th { class: "sticky left-0 top-0 z-30 bg-slate-50 dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 p-2 text-left min-w-56",
                            "裝置 / 感測器"
                        }
                        for k in keys.iter() {
                            th {
                                key: "{k}",
                                class: "sticky top-0 z-20 bg-slate-50 dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 p-2 text-left whitespace-nowrap",
                                "{k}"
                            }
                        }
                    }
                }
                tbody {
                    if owners.is_empty() {
                        tr {
                            td {
                                class: "p-4 text-slate-500 dark:text-slate-400",
                                colspan: "{colspan}",
                                "沒有符合的裝置/感測器"
                            }
                        }
                    } else if keys.is_empty() {
                        tr {
                            td {
                                class: "p-4 text-slate-500 dark:text-slate-400",
                                colspan: "{colspan}",
                                "此範圍內尚無任何屬性"
                            }
                        }
                    }
                    for o in owners.iter() {
                        MatrixRow {
                            key: "{owner_id_str(&o.owner)}",
                            owner_id: owner_id_str(&o.owner),
                            owner_label: o.label.clone(),
                            attrs: o.attrs.clone(),
                            keys: keys.clone(),
                            pending,
                            baseline_map: baseline_map.clone(),
                            compare_enabled,
                            is_baseline: compare_enabled && owner_id_str(&o.owner) == baseline_owner_id,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MatrixRow(
    owner_id: String,
    owner_label: String,
    attrs: Vec<Attribute>,
    keys: Vec<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_map: Option<HashMap<String, String>>,
    compare_enabled: bool,
    is_baseline: bool,
) -> Element {
    rsx! {
        tr {
            td { class: "sticky left-0 z-10 bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 p-2 whitespace-nowrap",
                span { "{owner_label}" }
                if is_baseline {
                    span { class: "ml-2 px-1.5 py-0.5 text-xs border rounded text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-400",
                        "基準"
                    }
                }
            }
            for k in keys.iter() {
                MatrixCell {
                    key: "{k}",
                    owner_id: owner_id.clone(),
                    attr_key: k.clone(),
                    original: attrs.iter().find(|a| &a.key == k).map(|a| a.value.clone()),
                    pending,
                    baseline_value: baseline_map.as_ref().and_then(|m| m.get(k).cloned()),
                    compare_enabled,
                    is_baseline,
                }
            }
        }
    }
}

#[component]
fn MatrixCell(
    owner_id: String,
    attr_key: String,
    original: Option<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_value: Option<String>,
    compare_enabled: bool,
    is_baseline: bool,
) -> Element {
    let cell_id = (owner_id, attr_key.clone());
    let (display_value, is_dirty) = cell_state(&cell_id, &original, pending);
    let differs_from_baseline = compare_enabled && !is_baseline && baseline_value != display_value;

    let on_change = {
        let cell_id = cell_id.clone();
        move |e: FormEvent| {
            pending.write().insert(cell_id.clone(), PendingOp::SetValue(e.value()));
        }
    };
    let on_clear = move |_| {
        pending.write().insert(cell_id.clone(), PendingOp::Delete);
    };

    rsx! {
        td {
            class: "p-1 align-top min-w-40 border",
            class: if differs_from_baseline { "border-amber-500 dark:border-amber-400 bg-amber-50 dark:bg-amber-950/40" } else { "border-slate-200 dark:border-zinc-800" },
            div { class: "flex items-center gap-1",
                DxInput {
                    class: "flex-1",
                    placeholder: if original.is_none() { "（未設定）" } else { "" },
                    value: display_value.clone().unwrap_or_default(),
                    onchange: on_change,
                }
                if is_dirty {
                    span {
                        class: "text-xs text-blue-600 dark:text-blue-400",
                        title: "尚未套用",
                        "●"
                    }
                }
                if display_value.is_some() {
                    button {
                        class: "text-xs text-red-600 dark:text-red-400 shrink-0",
                        r#type: "button",
                        onclick: on_clear,
                        "✕"
                    }
                }
            }
        }
    }
}

// ─── Mobile card layout (small screens) ──────────────────────────────────────
//
// The wide owner×key matrix is unusable on a phone — instead each owner becomes a collapsible
// card, and its attributes are stacked one-per-line (key label above the value input) so nothing
// needs horizontal scrolling.

#[component]
fn OwnerCardsMobile(
    owners: Vec<OwnerData>,
    keys: Vec<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_owner_id: String,
    compare_enabled: bool,
) -> Element {
    let baseline_map = build_baseline_map(&owners, &baseline_owner_id, compare_enabled);

    rsx! {
        div { class: "flex flex-col gap-3",
            if owners.is_empty() {
                div { class: "p-4 text-sm text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-zinc-800 rounded-lg",
                    "沒有符合的裝置/感測器"
                }
            } else if keys.is_empty() {
                div { class: "p-4 text-sm text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-zinc-800 rounded-lg",
                    "此範圍內尚無任何屬性"
                }
            }
            for o in owners.iter() {
                OwnerCardMobile {
                    key: "{owner_id_str(&o.owner)}",
                    owner_id: owner_id_str(&o.owner),
                    owner_label: o.label.clone(),
                    attrs: o.attrs.clone(),
                    keys: keys.clone(),
                    pending,
                    baseline_map: baseline_map.clone(),
                    compare_enabled,
                    is_baseline: compare_enabled && owner_id_str(&o.owner) == baseline_owner_id,
                }
            }
        }
    }
}

#[component]
fn OwnerCardMobile(
    owner_id: String,
    owner_label: String,
    attrs: Vec<Attribute>,
    keys: Vec<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_map: Option<HashMap<String, String>>,
    compare_enabled: bool,
    is_baseline: bool,
) -> Element {
    rsx! {
        details { class: "border border-slate-200 dark:border-zinc-800 rounded-lg overflow-hidden",
            summary { class: "p-3 cursor-pointer select-none flex items-center gap-2 bg-slate-50 dark:bg-zinc-900 text-sm font-medium",
                span { class: "flex-1", "{owner_label}" }
                if is_baseline {
                    span { class: "px-1.5 py-0.5 text-xs border rounded text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-400",
                        "基準"
                    }
                }
            }
            div { class: "flex flex-col divide-y divide-slate-200 dark:divide-zinc-800",
                for k in keys.iter() {
                    MobileAttributeRow {
                        key: "{k}",
                        owner_id: owner_id.clone(),
                        attr_key: k.clone(),
                        original: attrs.iter().find(|a| &a.key == k).map(|a| a.value.clone()),
                        pending,
                        baseline_value: baseline_map.as_ref().and_then(|m| m.get(k).cloned()),
                        compare_enabled,
                        is_baseline,
                    }
                }
            }
        }
    }
}

#[component]
fn MobileAttributeRow(
    owner_id: String,
    attr_key: String,
    original: Option<String>,
    pending: Signal<HashMap<(String, String), PendingOp>>,
    baseline_value: Option<String>,
    compare_enabled: bool,
    is_baseline: bool,
) -> Element {
    let cell_id = (owner_id, attr_key.clone());
    let (display_value, is_dirty) = cell_state(&cell_id, &original, pending);
    let differs_from_baseline = compare_enabled && !is_baseline && baseline_value != display_value;

    let on_change = {
        let cell_id = cell_id.clone();
        move |e: FormEvent| {
            pending.write().insert(cell_id.clone(), PendingOp::SetValue(e.value()));
        }
    };
    let on_clear = move |_| {
        pending.write().insert(cell_id.clone(), PendingOp::Delete);
    };

    rsx! {
        div {
            class: "p-2 flex flex-col gap-1",
            class: if differs_from_baseline { "bg-amber-50 dark:bg-amber-950/40" },
            span { class: "text-xs text-slate-500 dark:text-slate-400 font-mono break-all",
                "{attr_key}"
            }
            div { class: "flex items-center gap-1",
                DxInput {
                    class: "flex-1",
                    placeholder: if original.is_none() { "（未設定）" } else { "" },
                    value: display_value.clone().unwrap_or_default(),
                    onchange: on_change,
                }
                if is_dirty {
                    span {
                        class: "text-xs text-blue-600 dark:text-blue-400",
                        title: "尚未套用",
                        "●"
                    }
                }
                if display_value.is_some() {
                    button {
                        class: "text-xs text-red-600 dark:text-red-400 shrink-0",
                        r#type: "button",
                        onclick: on_clear,
                        "✕"
                    }
                }
            }
        }
    }
}

#[component]
fn CellDiffCard(diff: CellDiff) -> Element {
    let (label, class) = diff.kind.label_and_class();

    rsx! {
        div { class: "border border-slate-200 dark:border-zinc-800 rounded-lg p-3 flex items-center gap-3 flex-wrap",
            span { class: "px-1.5 py-0.5 border rounded text-xs shrink-0 {class}", "{label}" }
            span { class: "text-sm", "{diff.owner_label}" }
            span { class: "text-sm font-mono", "{diff.attr_key}" }
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
