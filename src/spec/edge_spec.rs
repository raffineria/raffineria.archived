use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "edge_spec")]
pub struct EdgeSpec {
    pub producer: PortSpec,
    pub consumer: PortSpec,
    pub schema: serde_json::Value,
}
