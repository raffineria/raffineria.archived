#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "port_spec")]
pub struct PortSpec {
    pub vertex: String,
    pub port: String,
}
