use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "vertex_spec")]
pub struct VertexSpec {
    pub run: RunSpec,

    #[serde(default)]
    pub inlets: Vec<String>,
    #[serde(default)]
    pub outlets: Vec<String>,

    #[serde(default)]
    pub restart_strategy: RestartStrategySpec,
}
