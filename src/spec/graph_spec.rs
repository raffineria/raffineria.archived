use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "graph_spec")]
pub struct GraphSpec {
    #[serde(default)]
    pub vertices: HashMap<String, VertexSpec>,
    #[serde(default)]
    pub edges: Vec<EdgeSpec>,
    #[serde(default)]
    pub inlets: Vec<PortSpec>,
    #[serde(default)]
    pub outlets: Vec<PortSpec>,
}
