use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use core::fmt;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
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

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
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

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Default)]
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

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct RawData {
    pub id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub value: Vec<String>,
    pub time: Option<String>,
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

    fn device(&self, device_id: &str) -> String {
        match self {
            Endpoint::General(endpoint) => endpoint.device(device_id),
            Endpoint::Edge(endpoint) => endpoint.device(device_id),
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
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct GeneralEndpoint {
    pub base_url: String,
}

pub trait EndpointTrait {
    fn active_notify(&self, device_id: &str) -> String;
    fn active_setting(&self, device_id: &str) -> String;
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

    fn device(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}", self.base_url)
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
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EdgeEndpoint {
    pub base_url: String,
}

impl EndpointTrait for EdgeEndpoint {
    fn sensor(&self, device_id: &str, sensor_id: &str) -> String {
        format!("{}/device/{device_id}/sensor/{sensor_id}", self.base_url)
    }

    fn metadata(&self) -> String {
        format!("{}/metadata", self.base_url)
    }

    fn rawdata(&self, device_id: &str) -> String {
        format!("{}/device/{device_id}/rawdata", self.base_url)
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
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Project {
    pub project_key: String,
    pub endpoint_key: String,
}

pub type Endpoints = HashMap<String, Endpoint>;
pub type Projects = HashMap<String, Project>;
