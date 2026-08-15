//! Project-level data/snapshot export: pick a date range + devices/sensors, then download a
//! CSV (or zip of CSVs) of raw sensor data, or a zip of snapshot images.
//!
//! Large snapshot exports are kept memory-bounded by flushing the in-progress zip into a new
//! "part" once it crosses [`PART_SIZE_LIMIT_BYTES`], and by handing bytes to the browser in
//! small base64 chunks (see [`download_binary_chunked`]) instead of one giant buffer/string.
//! Finished parts are not auto-downloaded (browsers block/throttle automatic multi-file
//! downloads) — they're listed with a manual "Download" button per part instead.

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_std::task::sleep;
use base64::prelude::*;
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use futures_util::{stream, StreamExt};
use dioxus_primitives::calendar::DateRange;
use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::Client;
use time::format_description::well_known::Iso8601;
use time::macros::{format_description, offset};
use time::{Date, OffsetDateTime};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{
    api::{ApiHelper, SensorRawDataQuery},
    components::{
        button::{Button, ButtonVariant},
        card::{Card, CardContent, CardHeader, CardTitle},
        checkbox::Checkbox,
        date_picker::DatePicker,
    },
    models::{Device, Endpoint, GetRawData, Sensor, SensorType},
    ui::{
        breadcrumb::{Breadcrumb, BreadcrumbItem},
        page_header::PageHeader,
    },
    views::{global::HeaderContext, sensor_v2::ProjectContext},
    Route,
};

/// Flush the in-progress zip into a new part once its accumulated (uncompressed) content
/// reaches this size. Tune down for lower-memory targets if needed.
const PART_SIZE_LIMIT_BYTES: u64 = 500 * 1024 * 1024;
/// Size of each base64 chunk sent to the browser per `eval.send()` call when downloading a
/// binary part, so no single contiguous string/buffer needs to hold a whole part at once.
const DOWNLOAD_CHUNK_BYTES: usize = 12 * 1024 * 1024;
/// Max in-flight snapshot fetches at once — parallel enough to be fast, capped so the export
/// doesn't look like a burst/DDoS to the server.
const SNAPSHOT_FETCH_CONCURRENCY: usize = 4;
/// Max retry attempts for a single snapshot fetch before giving up on it.
const SNAPSHOT_FETCH_MAX_RETRIES: u32 = 5;
/// Base delay before a snapshot fetch retry; doubles each attempt so repeated failures back off
/// instead of hammering the server.
const SNAPSHOT_FETCH_RETRY_BASE_DELAY: Duration = Duration::from_millis(300);

/// Fetches one snapshot's bytes, retrying transient failures up to
/// [`SNAPSHOT_FETCH_MAX_RETRIES`] times with exponential backoff.
async fn fetch_snapshot_bytes_with_retry(
    client: &Client,
    endpoint: &Endpoint,
    device_id: &str,
    sensor_id: &str,
    snapshot_id: &str,
    project_key: &str,
) -> Result<Vec<u8>> {
    let mut attempt = 0;
    loop {
        match ApiHelper::fetch_snapshot_bytes(
            client,
            endpoint,
            device_id,
            sensor_id,
            snapshot_id,
            project_key,
        )
        .await
        {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt >= SNAPSHOT_FETCH_MAX_RETRIES => {
                return Err(e).context(format!("已重試 {SNAPSHOT_FETCH_MAX_RETRIES} 次仍失敗"));
            }
            Err(_) => {
                attempt += 1;
                sleep(SNAPSHOT_FETCH_RETRY_BASE_DELAY * 2u32.pow(attempt - 1)).await;
            }
        }
    }
}

fn allow_all_sensors(_: &Sensor) -> bool {
    true
}

fn allow_snapshot_sensors(sensor: &Sensor) -> bool {
    sensor.kind == SensorType::Snapshot
}

#[derive(Clone, Copy, PartialEq)]
enum SensorFilter {
    All,
    SnapshotOnly,
}

impl SensorFilter {
    fn matches(self, sensor: &Sensor) -> bool {
        match self {
            SensorFilter::All => allow_all_sensors(sensor),
            SensorFilter::SnapshotOnly => allow_snapshot_sensors(sensor),
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_time_for_filename(time: &str) -> String {
    time.chars()
        .map(|c| match c {
            ':' | ' ' | '/' | '\\' => '-',
            c => c,
        })
        .collect()
}

/// UTC `[start, end)` bounds for a picked date range: `query_*` are full ISO-8601 (used for the
/// API request) and `file_*` are second-precision, filename-safe versions of the same instants.
struct DateRangeBounds {
    query_start: String,
    query_end: String,
    file_start: String,
    file_end: String,
}

/// Converts a picked [`DateRange`] into `[start, end)` UTC bounds, treating both ends as whole
/// GMT+8 days (matches the GMT+8 convention used by `SensorHistory`).
fn date_range_to_utc_bounds(range: DateRange) -> Result<DateRangeBounds> {
    let start_dt =
        OffsetDateTime::new_in_offset(range.start(), time::Time::MIDNIGHT, offset!(+8)).to_utc();
    let end_dt = OffsetDateTime::new_in_offset(range.end(), time::Time::MIDNIGHT, offset!(+8))
        .saturating_add(time::Duration::days(1))
        .to_utc();

    let filename_format = format_description!("[year][month][day]");
    Ok(DateRangeBounds {
        query_start: start_dt.format(&Iso8601::DATE_TIME_OFFSET)?,
        query_end: end_dt.format(&Iso8601::DATE_TIME_OFFSET)?,
        file_start: start_dt.format(&filename_format)?,
        file_end: end_dt.format(&filename_format)?,
    })
}

fn build_csv_bytes(rows: &[GetRawData]) -> Result<Vec<u8>> {
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer.write_record(["Time", "Value"])?;
    for row in rows {
        writer.write_record([row.time.as_str(), row.all_value().as_str()])?;
    }
    writer.flush()?;
    writer
        .into_inner()
        .map_err(|e| anyhow!("CSV writer flush failed: {e}"))
}

/// Incrementally builds a zip archive one entry at a time so a fetch loop can hand off a
/// finished "part" (and free its memory) as soon as [`PartWriter::should_flush`] reports the
/// accumulated (uncompressed) content has crossed the size limit — the caller never needs to
/// hold the whole dataset in memory at once, only whatever is in the current part.
struct PartWriter {
    writer: ZipWriter<Cursor<Vec<u8>>>,
    accumulated_bytes: u64,
    entries_in_part: usize,
    options: SimpleFileOptions,
}

impl PartWriter {
    fn new() -> Self {
        Self {
            writer: ZipWriter::new(Cursor::new(Vec::new())),
            accumulated_bytes: 0,
            entries_in_part: 0,
            options: SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        }
    }

    fn add_entry(&mut self, name: &str, data: &[u8]) -> Result<()> {
        self.writer.start_file(name, self.options)?;
        self.writer.write_all(data)?;
        self.accumulated_bytes += data.len() as u64;
        self.entries_in_part += 1;
        Ok(())
    }

    fn should_flush(&self, part_limit_bytes: u64) -> bool {
        self.accumulated_bytes >= part_limit_bytes
    }

    /// Finishes the current part and resets state for the next one. Returns `None` if the
    /// current part has no entries (nothing to flush).
    fn take_part(&mut self) -> Result<Option<Vec<u8>>> {
        if self.entries_in_part == 0 {
            return Ok(None);
        }
        let finished = std::mem::replace(self, Self::new());
        Ok(Some(finished.writer.finish()?.into_inner()))
    }
}

/// Pushes a finished part into `pending_parts` under a numbered filename (parts are always
/// numbered since, in a streaming builder, we don't know upfront whether there'll be just one).
fn push_part(
    mut pending_parts: Signal<Vec<PendingPart>>,
    base_name: &str,
    part_index: usize,
    bytes: Vec<u8>,
) {
    let filename = format!("{base_name}-part{part_index:02}.zip");
    pending_parts.write().push(PendingPart {
        filename,
        size_bytes: bytes.len(),
        bytes,
    });
}

/// Downloads `content` directly as a text file (used for the single-CSV, no-zip-needed case).
async fn download_text_file(content: String, filename: String, mime: &str) {
    let eval = document::eval(
        r#"
        let data = await dioxus.recv();
        let filename = await dioxus.recv();
        let mime = await dioxus.recv();
        const blob = new Blob([data], { type: mime });
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
    let _ = eval.send(content);
    let _ = eval.send(filename);
    let _ = eval.send(mime.to_string());
}

/// Downloads binary `bytes` (a finished zip part) by streaming base64 chunks to JS, which
/// assembles them into a `Blob` from an array of pieces rather than one giant buffer.
async fn download_binary_chunked(bytes: &[u8], filename: String, mime: &str) {
    let eval = document::eval(
        r#"
        let filename = await dioxus.recv();
        let mime = await dioxus.recv();
        const chunks = [];
        while (true) {
            const chunk = await dioxus.recv();
            if (chunk === null) break;
            const bin = atob(chunk);
            const arr = new Uint8Array(bin.length);
            for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
            chunks.push(arr);
        }
        const blob = new Blob(chunks, { type: mime });
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
    let _ = eval.send(filename);
    let _ = eval.send(mime.to_string());
    for chunk in bytes.chunks(DOWNLOAD_CHUNK_BYTES) {
        let _ = eval.send(Some(BASE64_STANDARD.encode(chunk)));
    }
    let _ = eval.send(None::<String>);
}

// ─── Device/sensor multi-select ──────────────────────────────────────────────

#[component]
fn DeviceSensorPicker(
    devices: ReadSignal<Vec<Device>>,
    sensor_filter: SensorFilter,
    selected: Signal<HashSet<(String, String)>>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-4",
            for device in devices() {
                DeviceSensorGroup {
                    key: "{device.id}",
                    device,
                    sensor_filter,
                    selected,
                }
            }
        }
    }
}

#[component]
fn DeviceSensorGroup(
    device: Device,
    sensor_filter: SensorFilter,
    selected: Signal<HashSet<(String, String)>>,
) -> Element {
    let device_id = device.id.clone();
    let sensors: Vec<Sensor> = device
        .sensors
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| sensor_filter.matches(s))
        .collect();

    if sensors.is_empty() {
        return rsx! {};
    }

    let sensor_ids: Vec<String> = sensors.iter().map(|s| s.id.clone()).collect();
    let state = {
        let device_id = device_id.clone();
        let sensor_ids = sensor_ids.clone();
        use_memo(move || {
            let sel = selected();
            let checked_count = sensor_ids
                .iter()
                .filter(|sid| sel.contains(&(device_id.clone(), (*sid).clone())))
                .count();
            if checked_count == 0 {
                CheckboxState::Unchecked
            } else if checked_count == sensor_ids.len() {
                CheckboxState::Checked
            } else {
                CheckboxState::Indeterminate
            }
        })
    };

    let on_toggle_all = {
        let device_id = device_id.clone();
        let sensor_ids = sensor_ids.clone();
        move |v: CheckboxState| {
            let should_check = v == CheckboxState::Checked;
            selected.with_mut(|sel| {
                for sid in &sensor_ids {
                    let key = (device_id.clone(), sid.clone());
                    if should_check {
                        sel.insert(key);
                    } else {
                        sel.remove(&key);
                    }
                }
            });
        }
    };

    rsx! {
        Card {
            CardHeader {
                div { class: "flex items-center gap-2",
                    Checkbox { checked: state(), on_checked_change: on_toggle_all }
                    CardTitle { {device.name.clone()} }
                }
            }
            CardContent {
                div { class: "flex flex-col gap-2",
                    for sensor in sensors {
                        SensorCheckboxRow {
                            key: "{sensor.id}",
                            device_id: device_id.clone(),
                            sensor,
                            selected,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SensorCheckboxRow(
    device_id: String,
    sensor: Sensor,
    selected: Signal<HashSet<(String, String)>>,
) -> Element {
    let key = (device_id.clone(), sensor.id.clone());
    let is_checked = {
        let key = key.clone();
        use_memo(move || selected().contains(&key))
    };
    let on_toggle = move |v: CheckboxState| {
        let key = key.clone();
        selected.with_mut(|sel| {
            if v == CheckboxState::Checked {
                sel.insert(key);
            } else {
                sel.remove(&key);
            }
        });
    };

    rsx! {
        label { class: "flex items-center gap-2 cursor-pointer",
            Checkbox {
                checked: if is_checked() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: on_toggle,
            }
            span { "{sensor.name}" }
            span { class: "text-xs text-slate-500 dark:text-slate-400", "({sensor.kind})" }
        }
    }
}

// ─── Finished zip parts, downloaded manually one at a time ───────────────────

struct PendingPart {
    filename: String,
    size_bytes: usize,
    bytes: Vec<u8>,
}

#[component]
fn PendingPartsList(pending_parts: Signal<Vec<PendingPart>>) -> Element {
    let count = pending_parts.read().len();
    if count == 0 {
        return rsx! {};
    }

    rsx! {
        div { class: "flex flex-col gap-2 mt-4",
            h2 { class: "text-lg font-bold", "已產生的壓縮檔" }
            for i in 0..count {
                PendingPartRow { key: "{i}", index: i, pending_parts }
            }
        }
    }
}

#[component]
fn PendingPartRow(index: usize, pending_parts: Signal<Vec<PendingPart>>) -> Element {
    let (filename, size_mb, is_downloaded) = {
        let parts = pending_parts.read();
        let part = &parts[index];
        (
            part.filename.clone(),
            part.size_bytes as f64 / (1024.0 * 1024.0),
            part.bytes.is_empty(),
        )
    };

    let on_download = move |_| async move {
        let (bytes, filename) = {
            let mut parts = pending_parts.write();
            let Some(part) = parts.get_mut(index) else {
                return;
            };
            (std::mem::take(&mut part.bytes), part.filename.clone())
        };
        if !bytes.is_empty() {
            download_binary_chunked(&bytes, filename, "application/zip").await;
        }
    };

    rsx! {
        Button {
            variant: ButtonVariant::Secondary,
            disabled: is_downloaded,
            onclick: on_download,
            Icon { icon: fa_solid_icons::FaFileZipper }
            if is_downloaded {
                "{filename}（已下載）"
            } else {
                "{filename}（{size_mb:.1} MB）— 下載"
            }
        }
    }
}

// ─── 匯出資料 (CSV) ───────────────────────────────────────────────────────────

#[component]
pub fn ExportDataPage(project_name: ReadSignal<String>) -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("匯出資料");
    });

    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;

    let mut start_date = use_signal(|| None::<Date>);
    let mut end_date = use_signal(|| None::<Date>);
    let selected = use_signal(HashSet::<(String, String)>::new);
    let mut is_exporting = use_signal(|| false);
    let mut progress = use_signal(|| (0usize, 0usize));
    let pending_parts = use_signal(Vec::<PendingPart>::new);

    let on_export = move |_| async move {
        let toast_api = use_toast();
        let (Some(start), Some(end)) = (start_date(), end_date()) else {
            toast_api.error(
                "請先選擇日期區間".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
            return;
        };
        let range = DateRange::new(start, end);
        let targets: Vec<(String, String)> = selected().into_iter().collect();
        if targets.is_empty() {
            toast_api.error(
                "請至少選擇一個感測器".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
            return;
        }
        let bounds = match date_range_to_utc_bounds(range) {
            Ok(v) => v,
            Err(e) => {
                toast_api.error(
                    "日期轉換失敗".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
                return;
            }
        };

        is_exporting.set(true);
        progress.set((0, targets.len()));

        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let devices = project_meta();

        let single_target = targets.len() == 1;
        let mut direct_download: Option<(String, Vec<u8>)> = None;
        let mut part_writer = PartWriter::new();
        let mut part_index = 0usize;
        let base_name = format!("{pk}-data-{}_{}", bounds.file_start, bounds.file_end);
        let mut any_success = false;
        let mut errors: Vec<String> = Vec::new();

        for (device_id, sensor_id) in targets {
            let found = devices.iter().find(|d| d.id == device_id).and_then(|d| {
                d.sensors
                    .iter()
                    .flatten()
                    .find(|s| s.id == sensor_id)
                    .map(|s| (d.clone(), s.clone()))
            });
            let Some((device, sensor)) = found else {
                progress.with_mut(|(done, _)| *done += 1);
                continue;
            };

            let result = ApiHelper::fetch_all_sensor_raw_data(
                &client,
                &ep,
                SensorRawDataQuery {
                    device_id: &device_id,
                    sensor_id: &sensor_id,
                    project_key: &pk,
                    start: &bounds.query_start,
                    end: &bounds.query_end,
                    order: "ASC",
                },
            )
            .await;

            match result.and_then(|rows| build_csv_bytes(&rows)) {
                Ok(bytes) => {
                    any_success = true;
                    let filename = format!(
                        "{}_{}_{}.csv",
                        sanitize_filename(&device.name),
                        sanitize_filename(&sensor.name),
                        sensor.id,
                    );
                    if single_target {
                        direct_download = Some((filename, bytes));
                    } else if let Err(e) = part_writer.add_entry(&filename, &bytes) {
                        errors.push(format!("{} - {}：壓縮失敗：{e}", device.name, sensor.name));
                    } else if part_writer.should_flush(PART_SIZE_LIMIT_BYTES) {
                        match part_writer.take_part() {
                            Ok(Some(bytes)) => {
                                part_index += 1;
                                push_part(pending_parts, &base_name, part_index, bytes);
                            }
                            Ok(None) => {}
                            Err(e) => errors.push(format!("壓縮失敗：{e}")),
                        }
                    }
                }
                Err(e) => errors.push(format!("{} - {}：{e}", device.name, sensor.name)),
            }

            progress.with_mut(|(done, _)| *done += 1);
        }

        if !any_success {
            toast_api.error(
                "匯出失敗".to_string(),
                ToastOptions::new()
                    .description("沒有任何感測器成功產生資料")
                    .duration(Duration::from_secs(10)),
            );
            is_exporting.set(false);
            return;
        }

        if let Some((filename, bytes)) = direct_download {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            download_text_file(text, filename, "text/csv").await;
        } else {
            match part_writer.take_part() {
                Ok(Some(bytes)) => {
                    part_index += 1;
                    push_part(pending_parts, &base_name, part_index, bytes);
                }
                Ok(None) => {}
                Err(e) => errors.push(format!("壓縮失敗：{e}")),
            }
        }

        if errors.is_empty() {
            toast_api.success(
                "匯出完成".to_string(),
                ToastOptions::new().duration(Duration::from_secs(10)),
            );
        } else {
            toast_api.error(
                format!("匯出完成，但有 {} 個感測器失敗", errors.len()),
                ToastOptions::new()
                    .description(errors.join("\n"))
                    .duration(Duration::from_secs(15)),
            );
        }

        is_exporting.set(false);
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
        PageHeader { title: "匯出資料" }

        div { class: "flex flex-col gap-4 mb-8",
            h2 { class: "text-lg font-bold", "日期區間" }
            div { class: "flex gap-4",
                DatePicker {
                    selected_date: start_date,
                    on_value_change: move |v| start_date.set(v),
                }
                DatePicker {
                    selected_date: end_date,
                    on_value_change: move |v| end_date.set(v),
                }
            }
        }

        div { class: "flex flex-col gap-4 mb-8",
            h2 { class: "text-lg font-bold", "選擇感測器" }
            if project_meta().is_empty() {
                p { "沒有裝置" }
            } else {
                DeviceSensorPicker {
                    devices: project_meta,
                    sensor_filter: SensorFilter::All,
                    selected,
                }
            }
        }

        div { class: "flex items-center gap-4",
            Button { disabled: is_exporting(), onclick: on_export,
                Icon { icon: fa_solid_icons::FaFileCsv }
                "匯出 CSV"
            }
            if is_exporting() {
                p {
                    {
                        let (done, total) = progress();
                        format!("已完成 {done}/{total}")
                    }
                }
            }
        }

        PendingPartsList { pending_parts }
        div { class: "h-48" }
    }
}

// ─── 匯出快照 (Snapshot images zip) ───────────────────────────────────────────

#[component]
pub fn ExportSnapshotsPage(project_name: ReadSignal<String>) -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("匯出快照");
    });

    let ctx: ProjectContext = use_context();
    let ProjectContext {
        project,
        endpoint,
        project_meta,
    } = ctx;

    let mut start_date = use_signal(|| None::<Date>);
    let mut end_date = use_signal(|| None::<Date>);
    let selected = use_signal(HashSet::<(String, String)>::new);
    let mut is_exporting = use_signal(|| false);
    let mut progress = use_signal(|| (0usize, 0usize));
    let pending_parts = use_signal(Vec::<PendingPart>::new);

    let on_export = move |_| async move {
        let toast_api = use_toast();
        let (Some(start), Some(end)) = (start_date(), end_date()) else {
            toast_api.error(
                "請先選擇日期區間".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
            return;
        };
        let range = DateRange::new(start, end);
        let targets: Vec<(String, String)> = selected().into_iter().collect();
        if targets.is_empty() {
            toast_api.error(
                "請至少選擇一個快照感測器".to_string(),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
            return;
        }
        let bounds = match date_range_to_utc_bounds(range) {
            Ok(v) => v,
            Err(e) => {
                toast_api.error(
                    "日期轉換失敗".to_string(),
                    ToastOptions::new()
                        .description(format!("{e}"))
                        .duration(Duration::from_secs(10)),
                );
                return;
            }
        };

        is_exporting.set(true);
        progress.set((0, 0));

        let client = Client::new();
        let ep = endpoint();
        let pk = project().project_key;
        let devices = project_meta();

        // Phase 1: list every snapshot reference in range across all selected sensors so the
        // progress bar's total is known before we start downloading image bytes.
        let mut snapshot_refs: Vec<(Device, Sensor, String, String)> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for (device_id, sensor_id) in &targets {
            let found = devices.iter().find(|d| &d.id == device_id).and_then(|d| {
                d.sensors
                    .iter()
                    .flatten()
                    .find(|s| &s.id == sensor_id)
                    .map(|s| (d.clone(), s.clone()))
            });
            let Some((device, sensor)) = found else {
                continue;
            };

            let result = ApiHelper::fetch_all_sensor_raw_data(
                &client,
                &ep,
                SensorRawDataQuery {
                    device_id,
                    sensor_id,
                    project_key: &pk,
                    start: &bounds.query_start,
                    end: &bounds.query_end,
                    order: "ASC",
                },
            )
            .await;

            match result {
                Ok(rows) => {
                    for row in rows {
                        // Snapshot values are stored as an 11-char prefix + snapshot id.
                        if let Some(Some(first)) = row.value.first() {
                            if first.len() > 11 {
                                let snapshot_id = first[11..].to_string();
                                snapshot_refs.push((
                                    device.clone(),
                                    sensor.clone(),
                                    row.time.clone(),
                                    snapshot_id,
                                ));
                            }
                        }
                    }
                }
                Err(e) => errors.push(format!("{} - {} 清單抓取失敗：{e}", device.name, sensor.name)),
            }
        }

        let total = snapshot_refs.len();
        progress.set((0, total));

        if total == 0 {
            toast_api.error(
                "沒有可匯出的快照".to_string(),
                ToastOptions::new()
                    .description(if errors.is_empty() {
                        String::new()
                    } else {
                        errors.join("\n")
                    })
                    .duration(Duration::from_secs(10)),
            );
            is_exporting.set(false);
            return;
        }

        // Phase 2: fetch each image's bytes and stream it straight into the current zip part,
        // flushing (and freeing) a part to `pending_parts` as soon as it crosses the size
        // limit — so we never hold more than one part's worth of raw image bytes at a time.
        let mut part_writer = PartWriter::new();
        let mut part_index = 0usize;
        let base_name = format!("{pk}-snapshots-{}_{}", bounds.file_start, bounds.file_end);
        let mut any_success = false;

        // Fetches run with bounded concurrency; the zip write stays sequential as each result
        // arrives since `PartWriter` isn't safe to write to concurrently.
        let mut fetches = stream::iter(snapshot_refs.into_iter().map(|(device, sensor, time, snapshot_id)| {
            let client = client.clone();
            let ep = ep.clone();
            let pk = pk.clone();
            async move {
                let result = fetch_snapshot_bytes_with_retry(
                    &client,
                    &ep,
                    &device.id,
                    &sensor.id,
                    &snapshot_id,
                    &pk,
                )
                .await;
                (device, sensor, time, snapshot_id, result)
            }
        }))
        .buffer_unordered(SNAPSHOT_FETCH_CONCURRENCY);

        while let Some((device, sensor, time, snapshot_id, result)) = fetches.next().await {
            match result {
                Ok(bytes) => {
                    let path = format!(
                        "{}/{}/{}_{}.jpg",
                        sanitize_filename(&device.name),
                        sanitize_filename(&sensor.name),
                        sanitize_time_for_filename(&time),
                        snapshot_id,
                    );
                    if let Err(e) = part_writer.add_entry(&path, &bytes) {
                        errors.push(format!(
                            "{} - {} @ {time} 壓縮失敗：{e}",
                            device.name, sensor.name
                        ));
                    } else {
                        any_success = true;
                        if part_writer.should_flush(PART_SIZE_LIMIT_BYTES) {
                            match part_writer.take_part() {
                                Ok(Some(bytes)) => {
                                    part_index += 1;
                                    push_part(pending_parts, &base_name, part_index, bytes);
                                }
                                Ok(None) => {}
                                Err(e) => errors.push(format!("壓縮失敗：{e}")),
                            }
                        }
                    }
                }
                Err(e) => errors.push(format!(
                    "{} - {} @ {time} 圖片下載失敗：{e}",
                    device.name, sensor.name
                )),
            }

            progress.with_mut(|(done, _)| *done += 1);
        }

        if !any_success {
            toast_api.error(
                "匯出失敗".to_string(),
                ToastOptions::new()
                    .description("沒有任何圖片下載成功")
                    .duration(Duration::from_secs(10)),
            );
            is_exporting.set(false);
            return;
        }

        match part_writer.take_part() {
            Ok(Some(bytes)) => {
                part_index += 1;
                push_part(pending_parts, &base_name, part_index, bytes);
            }
            Ok(None) => {}
            Err(e) => errors.push(format!("壓縮失敗：{e}")),
        }

        if errors.is_empty() {
            toast_api.success(
                "匯出完成，請於下方點擊下載".to_string(),
                ToastOptions::new().duration(Duration::from_secs(10)),
            );
        } else {
            toast_api.error(
                format!("匯出完成，但有 {} 筆失敗", errors.len()),
                ToastOptions::new()
                    .description(errors.join("\n"))
                    .duration(Duration::from_secs(15)),
            );
        }

        is_exporting.set(false);
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
        PageHeader { title: "匯出快照" }

        div { class: "flex flex-col gap-4 mb-8",
            h2 { class: "text-lg font-bold", "日期區間" }
            div { class: "flex gap-4",
                DatePicker {
                    selected_date: start_date,
                    on_value_change: move |v| start_date.set(v),
                }
                DatePicker {
                    selected_date: end_date,
                    on_value_change: move |v| end_date.set(v),
                }
            }
        }

        div { class: "flex flex-col gap-4 mb-8",
            h2 { class: "text-lg font-bold", "選擇快照感測器" }
            if project_meta().is_empty() {
                p { "沒有裝置" }
            } else {
                DeviceSensorPicker {
                    devices: project_meta,
                    sensor_filter: SensorFilter::SnapshotOnly,
                    selected,
                }
            }
        }

        div { class: "flex items-center gap-4",
            Button { disabled: is_exporting(), onclick: on_export,
                Icon { icon: fa_solid_icons::FaImages }
                "匯出快照"
            }
            if is_exporting() {
                p {
                    {
                        let (done, total) = progress();
                        format!("已完成 {done}/{total}")
                    }
                }
            }
        }

        PendingPartsList { pending_parts }
        div { class: "h-48" }
    }
}
