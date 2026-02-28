use std::collections::HashMap;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SensorWithData {
    pub sensor: Sensor,
    pub data: Option<RawData>,
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
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct GeneralEndpoint {
    pub base_url: String,
}

pub trait EndpointTrait {
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
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Project {
    pub project_key: String,
    pub endpoint_key: String,
}

pub type Endpoints = HashMap<String, Endpoint>;
pub type Projects = HashMap<String, Project>;
