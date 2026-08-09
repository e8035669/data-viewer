//! Centralized helpers for every HTTP call that talks to an [`Endpoint`].
//!
//! Any function that needs a [`Client`] together with an [`Endpoint`] to reach the backend
//! should live here instead of being inlined inside a view/component.

use std::str::FromStr;

use anyhow::Result;
use base64::prelude::*;
use reqwest::{Client, Url};

use crate::models::{
    ActiveDevice, ActiveInfo, ActiveNotify, Device, EditDevice, EditSensor, Endpoint,
    EndpointTrait, GetRawData, RawData, Rule1, Sensor,
};

pub struct ApiHelper;

/// Query params for [`ApiHelper::fetch_sensor_raw_data`].
pub struct SensorRawDataQuery<'a> {
    pub device_id: &'a str,
    pub sensor_id: &'a str,
    pub project_key: &'a str,
    pub start: &'a str,
    pub end: &'a str,
    pub order: &'a str,
}

impl ApiHelper {
    // ─── Device ──────────────────────────────────────────────────────────

    pub async fn req_project_meta(
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

    pub async fn fetch_all_devices(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
    ) -> Result<Vec<Device>> {
        let url = endpoint.all_device();
        let data = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Vec<Device>>()
            .await?;
        Ok(data)
    }

    pub async fn create_device(
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

    pub async fn update_device(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        edit_device: &EditDevice,
    ) -> Result<String> {
        let url = endpoint.device(device_id);
        let ret = client
            .put(url)
            .header("CK", project_key)
            .json(edit_device)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    pub async fn delete_device(
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

    // ─── Sensor ──────────────────────────────────────────────────────────

    pub async fn create_sensor(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        new_sensor: &Sensor,
    ) -> Result<String> {
        let url = endpoint.all_sensor(device_id);
        let ret = client
            .post(url)
            .header("CK", project_key)
            .json(new_sensor)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    pub async fn update_sensor(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        sensor_id: &str,
        edit_sensor: &EditSensor,
    ) -> Result<String> {
        let url = endpoint.sensor(device_id, sensor_id);
        let ret = client
            .put(url)
            .header("CK", project_key)
            .json(edit_sensor)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    pub async fn delete_sensor(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        sensor_id: &str,
    ) -> Result<()> {
        let url = endpoint.sensor(device_id, sensor_id);
        let _ret = client
            .delete(url)
            .header("CK", project_key)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    // ─── Raw data / snapshot ─────────────────────────────────────────────

    pub async fn fetch_raw_data(
        client: &Client,
        endpoint: &Endpoint,
        device_id: &str,
        project_key: &str,
    ) -> Result<Vec<RawData>> {
        let url = endpoint.rawdata(device_id);
        let ret = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Vec<RawData>>()
            .await?;
        Ok(ret)
    }

    pub async fn fetch_sensor_raw_data(
        client: &Client,
        endpoint: &Endpoint,
        query: SensorRawDataQuery<'_>,
    ) -> Result<Vec<GetRawData>> {
        let url = endpoint.sensor_rawdata(query.device_id, query.sensor_id);
        let mut url = Url::from_str(&url)?;
        url.query_pairs_mut()
            .append_pair("start", query.start)
            .append_pair("end", query.end)
            .append_pair("order", query.order);
        let ret = client
            .get(url)
            .header("CK", query.project_key)
            .send()
            .await?
            .json::<Vec<GetRawData>>()
            .await?;
        Ok(ret)
    }

    pub async fn fetch_snapshot_base64(
        client: &Client,
        endpoint: &Endpoint,
        device_id: &str,
        sensor_id: &str,
        snapshot_id: &str,
        project_key: &str,
    ) -> Result<String> {
        let url = endpoint.snapshot(device_id, sensor_id, snapshot_id);
        let img = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .bytes()
            .await?;
        let img_b64 = String::from("data:image/jpeg;base64,") + &BASE64_STANDARD.encode(img);
        Ok(img_b64)
    }

    // ─── Active monitor ──────────────────────────────────────────────────

    pub async fn fetch_active_info(
        client: &Client,
        endpoint: &Endpoint,
        device_id: &str,
        project_key: &str,
    ) -> Result<Option<ActiveInfo>> {
        let url = endpoint.active(device_id);
        let ret = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Option<ActiveInfo>>()
            .await?;
        Ok(ret)
    }

    pub async fn fetch_active_setting(
        client: &Client,
        endpoint: &Endpoint,
        device_id: &str,
        project_key: &str,
    ) -> Result<ActiveDevice> {
        let url = endpoint.active_setting(device_id);
        let ret = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<ActiveDevice>()
            .await?;
        Ok(ret)
    }

    pub async fn update_active_setting(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        setting: &ActiveDevice,
    ) -> Result<String> {
        let url = endpoint.active_setting(device_id);
        let ret = client
            .post(url)
            .header("CK", project_key)
            .json(setting)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    pub async fn fetch_active_notifies(
        client: &Client,
        endpoint: &Endpoint,
        device_id: &str,
        project_key: &str,
    ) -> Result<Vec<ActiveNotify>> {
        let url = endpoint.active_notify(device_id);
        let ret = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Vec<ActiveNotify>>()
            .await?;
        Ok(ret)
    }

    /// Creates/updates an active notify setting. Returns the response status together with the
    /// response body so callers can decide how to report failures.
    pub async fn upsert_active_notify(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        notify: &ActiveNotify,
    ) -> Result<(reqwest::StatusCode, String)> {
        let url = endpoint.active_notify(device_id);
        let resp = client
            .post(url)
            .header("CK", project_key)
            .json(notify)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        Ok((status, text))
    }

    pub async fn delete_active_notify(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
        device_id: &str,
        notify_id: i32,
    ) -> Result<String> {
        let url = endpoint.active_notify_delete(device_id, notify_id);
        let ret = client
            .delete(url)
            .header("CK", project_key)
            .send()
            .await?
            .text()
            .await?;
        Ok(ret)
    }

    // ─── Rules ───────────────────────────────────────────────────────────

    pub async fn fetch_rules(
        client: &Client,
        endpoint: &Endpoint,
        project_key: &str,
    ) -> Result<Vec<Rule1>> {
        let url = endpoint.all_expression();
        let ret = client
            .get(url)
            .header("CK", project_key)
            .send()
            .await?
            .json::<Vec<Rule1>>()
            .await?;
        Ok(ret)
    }
}
