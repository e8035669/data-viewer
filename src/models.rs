use anyhow::{anyhow, Result};
use core::fmt;
use dioxus::prelude::*;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumIter};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Project {
    pub project_key: String,
    pub endpoint_key: String,
}

pub type Endpoints = HashMap<String, Endpoint>;
pub type Projects = HashMap<String, Project>;
pub type AuthInfos = HashMap<String, AuthInfo>;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default, EnumIter, Display)]
#[serde(rename_all = "lowercase")]
pub enum SensorType {
    #[default]
    Gauge,
    Text,
    Switch,
    Snapshot,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default, Store)]
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

impl Sensor {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            desc: Some(String::new()),
            kind: SensorType::Gauge,
            attributes: Some(Vec::new()),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default, Store)]
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

impl EditSensor {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            desc: Some(String::new()),
            kind: SensorType::Gauge,
            attributes: Some(Vec::new()),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Store, Default)]
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

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Store, Default)]
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

impl EditDevice {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            desc: Some(String::new()),
            kind: "general".to_string(),
            uri: Some(String::new()),
            attributes: Some(Vec::new()),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct DeviceResponse {
    pub id: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct RawData {
    pub id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub value: Vec<Option<String>>,
    pub time: Option<String>,
}

impl RawData {
    pub fn all_value(&self) -> String {
        self.value
            .iter()
            .map(|v| v.clone().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct GetRawData {
    pub id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub value: Vec<Option<String>>,
    pub time: String,
}

impl GetRawData {
    pub fn all_value(&self) -> String {
        self.value
            .iter()
            .map(|v| v.clone().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SensorWithData {
    pub sensor: Sensor,
    pub data: Option<RawData>,
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActiveStatus {
    #[default]
    Unset,
    Start,
    Online,
    Offline,
    Stop,
    Abnormal,
}

impl fmt::Display for ActiveStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActiveStatus::Unset => write!(f, "Unset"),
            ActiveStatus::Start => write!(f, "Start"),
            ActiveStatus::Online => write!(f, "Online"),
            ActiveStatus::Offline => write!(f, "Offline"),
            ActiveStatus::Stop => write!(f, "Stop"),
            ActiveStatus::Abnormal => write!(f, "Abnormal"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct ActiveInfo {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub status: ActiveStatus,
    pub record: Option<i32>,
    #[serde(rename = "lastDataTime")]
    pub last_data_time: Option<String>,
    #[serde(rename = "createTime")]
    pub create_time: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]

pub struct ActiveDevice {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub enable: bool,
    pub period: String,
    #[serde(rename = "minUploads")]
    pub min_uploads: Option<i32>,
    #[serde(rename = "maxUploads")]
    pub max_uploads: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor: Option<String>,
    #[serde(rename = "createTime")]
    pub create_time: Option<u64>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]

pub struct ActiveNotifySetting {
    pub to: String,
    pub message: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]

pub struct ActiveNotify {
    pub id: i32,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub enable: bool,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub setting: ActiveNotifySetting,
    #[serde(rename = "createTime")]
    pub create_time: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Endpoint {
    General(GeneralEndpoint),
    Edge(EdgeEndpoint),
}

impl EndpointTrait for Endpoint {
    fn metadata(&self) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.metadata(),
            Endpoint::Edge(endpoint) => endpoint.metadata(),
        }
    }

    fn rawdata(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.rawdata(device_id),
            Endpoint::Edge(endpoint) => endpoint.rawdata(device_id),
        }
    }

    fn snapshot(&self, device_id: &str, sensor_id: &str, snapshot_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.snapshot(device_id, sensor_id, snapshot_id),
            Endpoint::Edge(endpoint) => endpoint.snapshot(device_id, sensor_id, snapshot_id),
        }
    }

    fn baseurl(&self) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.baseurl(),
            Endpoint::Edge(endpoint) => endpoint.baseurl(),
        }
    }

    fn kind(&self) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.kind(),
            Endpoint::Edge(endpoint) => endpoint.kind(),
        }
    }

    fn all_device(&self) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.all_device(),
            Endpoint::Edge(endpoint) => endpoint.all_device(),
        }
    }

    fn device(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.device(device_id),
            Endpoint::Edge(endpoint) => endpoint.device(device_id),
        }
    }

    fn all_sensor(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.all_sensor(device_id),
            Endpoint::Edge(endpoint) => endpoint.all_sensor(device_id),
        }
    }

    fn sensor(&self, device_id: &str, sensor_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.sensor(device_id, sensor_id),
            Endpoint::Edge(endpoint) => endpoint.sensor(device_id, sensor_id),
        }
    }

    fn active_notify(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.active_notify(device_id),
            Endpoint::Edge(endpoint) => endpoint.active_notify(device_id),
        }
    }

    fn active_setting(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.active_setting(device_id),
            Endpoint::Edge(endpoint) => endpoint.active_setting(device_id),
        }
    }

    fn active(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.active(device_id),
            Endpoint::Edge(endpoint) => endpoint.active(device_id),
        }
    }

    fn active_notify_delete(&self, device_id: &str, notify_id: i32) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.active_notify_delete(device_id, notify_id),
            Endpoint::Edge(endpoint) => endpoint.active_notify_delete(device_id, notify_id),
        }
    }

    fn sensor_rawdata(&self, device_id: &str, sensor_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.sensor_rawdata(device_id, sensor_id),
            Endpoint::Edge(endpoint) => endpoint.sensor_rawdata(device_id, sensor_id),
        }
    }

    fn expression(&self, expression_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.expression(expression_id),
            Endpoint::Edge(endpoint) => endpoint.expression(expression_id),
        }
    }

    fn all_expression(&self) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.all_expression(),
            Endpoint::Edge(endpoint) => endpoint.all_expression(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct GeneralEndpoint {
    pub base_url: String,
}

pub trait EndpointTrait {
    fn all_expression(&self) -> String;
    fn expression(&self, expression_id: &str) -> String;
    fn all_sensor(&self, device_id: &str) -> String;
    fn all_device(&self) -> String;
    fn sensor_rawdata(&self, device_id: &str, sensor_id: &str) -> String;
    fn active_notify(&self, device_id: &str) -> String;
    fn active_setting(&self, device_id: &str) -> String;
    fn active_notify_delete(&self, device_id: &str, notify_id: i32) -> String;
    fn active(&self, device_id: &str) -> String;
    fn sensor(&self, device_id: &str, sensor_id: &str) -> String;
    fn metadata(&self) -> String;
    fn rawdata(&self, device_id: &str) -> String;
    fn snapshot(&self, device_id: &str, sensor_id: &str, snapshot_id: &str) -> String;
    fn baseurl(&self) -> String;
    fn kind(&self) -> String;
    fn device(&self, device_id: &str) -> String;
}

impl EndpointTrait for GeneralEndpoint {
    fn metadata(&self) -> String {
        format!("{}/metadata", self.base_url)
    }
    fn rawdata(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/rawdata", self.base_url)
    }

    fn sensor_rawdata(&self, device_id: &str, sensor_id: &str) -> String {
        format!(
            "{}/device/{device_id}/sensor/{sensor_id}/rawdata",
            self.base_url
        )
    }

    fn snapshot(&self, device_id: &str, sensor_id: &str, snapshot_id: &str) -> String {
        format!(
            "{}/device/{device_id}/sensor/{sensor_id}/snapshot/{snapshot_id}",
            self.base_url
        )
    }

    fn baseurl(&self) -> String {
        self.base_url.to_owned()
    }

    fn kind(&self) -> String {
        "General".to_string()
    }

    fn all_device(&self) -> String {
        format!("{}/device", self.base_url)
    }

    fn device(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}", self.base_url)
    }

    fn all_sensor(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/sensor", self.base_url)
    }

    fn sensor(&self, device_id: &str, sensor_id: &str) -> String {
        format!("{}/device/{device_id}/sensor/{sensor_id}", self.base_url)
    }

    fn active(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active", self.base_url)
    }

    fn active_setting(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active/setting", self.base_url)
    }

    fn active_notify(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active/notify", self.base_url)
    }

    fn active_notify_delete(&self, device_id: &str, notify_id: i32) -> String {
        format!(
            "{}/device/{device_id}/active/notify/{notify_id}",
            self.base_url
        )
    }

    fn expression(&self, expression_id: &str) -> String {
        format!("{}/expression/{expression_id}", self.base_url)
    }

    fn all_expression(&self) -> String {
        format!("{}/expression", self.base_url)
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EdgeEndpoint {
    pub base_url: String,
}

impl EndpointTrait for EdgeEndpoint {
    fn all_sensor(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/sensor", self.base_url)
    }

    fn sensor(&self, device_id: &str, sensor_id: &str) -> String {
        format!("{}/device/{device_id}/sensor/{sensor_id}", self.base_url)
    }

    fn metadata(&self) -> String {
        format!("{}/metadata", self.base_url)
    }

    fn rawdata(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/rawdata", self.base_url)
    }

    fn sensor_rawdata(&self, device_id: &str, sensor_id: &str) -> String {
        format!(
            "{}/device/{device_id}/sensor/{sensor_id}/rawdata",
            self.base_url
        )
    }

    fn snapshot(&self, device_id: &str, sensor_id: &str, snapshot_id: &str) -> String {
        format!(
            "{}/snapshot/device/{device_id}/sensor/{sensor_id}/snapshot/{snapshot_id}",
            self.base_url
        )
    }

    fn baseurl(&self) -> String {
        self.base_url.to_owned()
    }

    fn kind(&self) -> String {
        "Edge".to_string()
    }

    fn all_device(&self) -> String {
        format!("{}/device", self.base_url)
    }

    fn device(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}", self.base_url)
    }

    fn active_notify(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active/notify", self.base_url)
    }

    fn active_setting(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active/setting", self.base_url)
    }

    fn active(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/active", self.base_url)
    }

    fn active_notify_delete(&self, device_id: &str, notify_id: i32) -> String {
        format!(
            "{}/device/{device_id}/active/notify/{notify_id}",
            self.base_url
        )
    }

    fn expression(&self, expression_id: &str) -> String {
        format!("{}/expression/{expression_id}", self.base_url)
    }

    fn all_expression(&self) -> String {
        format!("{}/expression", self.base_url)
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rule1 {
    pub id: Option<i32>,
    pub name: String,
    pub desc: String,
    pub expression: String,
    pub devices: Vec<String>,
    pub sensor: String,
    pub enable: String,
    pub project: String,
    pub mode: RuleMode,
    #[serde(rename = "type")]
    pub r#type: RuleType,
    pub actions: Vec<Action>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub action_type: ActionType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_event: Option<EmailEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_event: Option<DeviceEvent>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct EmailEvent {
    pub email: String,
    pub subject: String,
    pub content: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct DeviceEvent {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "sensorId")]
    pub sensor_id: String,
    #[serde(rename = "type")]
    pub kind: DeviceEventType,
    pub value: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceEventType {
    #[default]
    Rawdata,
    Cmd,
    Ack,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    #[default]
    EventAction,
    RecoverAction,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum RuleType {
    #[serde(rename = "DATA")]
    #[default]
    Data,
    #[serde(rename = "TIME")]
    Time,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleMode {
    #[default]
    Single,
    Continue,
    FixedRate,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rule2 {
    pub id: Option<String>,
    pub name: String,
    pub desc: String,
    pub project_id: String,
    pub targets: std::collections::HashMap<String, Vec<String>>,
    pub formulas: Vec<Formula>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Formula {
    pub formula: String,
    pub type_num: String,
    pub type_unit: String,
    pub actions: Vec<FormulaAction>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FormulaAction {
    pub action_type: String,
    pub device_id: String,
    pub sensor_id: String,
    #[serde(rename = "CK")]
    pub ck: String,
    pub material_type: DeviceEventType,
    pub values: Vec<String>,
}

// Project API request

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct GeneralAuthInfo {
    pub endpoint: String,
    pub x_api_key: String,
}

impl GeneralAuthInfo {
    fn new(endpoint: &str, x_api_key: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            x_api_key: x_api_key.to_string(),
        }
    }

    pub async fn get_partial_projects(
        client: &Client,
        base_url: &str,
        api_key: &str,
    ) -> Result<Vec<PartialProjectResp>> {
        let url = format!("{base_url}/project");
        let resp = client.get(&url).header("X-API-KEY", api_key).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("General get_projects failed ({status}): {body}"));
        }

        Ok(resp.json::<Vec<PartialProjectResp>>().await?)
    }

    pub async fn get_project_detail(
        client: &Client,
        base_url: &str,
        api_key: &str,
        project_id: &str,
    ) -> Result<ProjectResp> {
        let url = format!("{base_url}/project/{project_id}");
        let resp = client.get(&url).header("X-API-KEY", api_key).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "General get_project({project_id}) failed ({status}): {body}"
            ));
        }

        Ok(resp.json::<ProjectResp>().await?)
    }

    pub async fn get_projects(&self) -> Result<Vec<ProjectResp>> {
        let client = Client::new();
        let partial_projects =
            GeneralAuthInfo::get_partial_projects(&client, &self.endpoint, &self.x_api_key).await?;
        let futs: Vec<_> = partial_projects
            .iter()
            .map(|p| {
                GeneralAuthInfo::get_project_detail(&client, &self.endpoint, &self.x_api_key, &p.id)
            })
            .collect();

        let results = futures_util::future::join_all(futs).await;
        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]

pub struct EdgeAuthInfo {
    pub url: String,
    pub digest: String,
}

impl EdgeAuthInfo {
    fn new(url: &str, digest: &str) -> Self {
        Self {
            url: url.to_string(),
            digest: digest.to_string(),
        }
    }

    pub async fn edge_get_auth(
        client: &Client,
        base_url: &str,
        username: &str,
        digest: &str,
    ) -> Result<AuthResponse> {
        let url = format!("{base_url}/iot/v1/auth");
        let resp = client
            .get(&url)
            .query(&[("username", username), ("ttl", "600")])
            .header("digest", digest)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Edge auth failed ({status}): {body}"));
        }

        Ok(resp.json::<AuthResponse>().await?)
    }

    pub async fn edge_get_project(
        client: &Client,
        base_url: &str,
        token: &str,
    ) -> Result<EdgeProjectResp> {
        let url = format!("{base_url}/iot/v1/project");
        let resp = client.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Edge get_project failed ({status}): {body}"));
        }

        Ok(resp.json::<EdgeProjectResp>().await?)
    }

    pub async fn get_projects(&self) -> Result<Vec<ProjectResp>> {
        let url = Url::parse(&self.url)?;
        let base_url = format!(
            "{}://{}",
            url.scheme(),
            url.host_str().ok_or_else(|| anyhow!("missing host"))?
        );
        let username = url
            .query_pairs()
            .find(|(k, _)| k == "username")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| anyhow!("missing username in query"))?;

        let client = Client::new();
        let auth = EdgeAuthInfo::edge_get_auth(&client, &base_url, &username, &self.digest).await?;
        let edge_resp =
            EdgeAuthInfo::edge_get_project(&client, &base_url, &auth.result.access_token).await?;

        Ok(edge_resp.result)
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub enum AuthInfo {
    General(GeneralAuthInfo),
    Edge(EdgeAuthInfo),
}

impl AuthInfo {
    pub async fn get_projects(&self) -> Result<Vec<ProjectResp>> {
        match self {
            AuthInfo::General(info) => info.get_projects().await,
            AuthInfo::Edge(info) => info.get_projects().await,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct DigestInfo {
    pub digest: String,
}

// Project API response

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub result: AuthorityInfo,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityInfo {
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    #[default]
    Admin,
    ReadOnly,
    ReadWrite,
    WriteRawdataOnly,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectKey {
    pub key: String,
    pub permission: Permission,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialProjectResp {
    pub id: String,
    pub name: String,
    pub desc: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResp {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub project_keys: Vec<ProjectKey>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EdgeProjectResp {
    pub result: Vec<ProjectResp>,
}
