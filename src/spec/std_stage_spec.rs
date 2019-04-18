#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "std_stage_spec")]
pub enum StdStageSpec {
    #[serde(rename = "tee")]
    Tee {
        schema: serde_json::Value,
        outlets_count: usize,
    },

    #[serde(rename = "merge")]
    Merge {
        #[serde(default = "default_eagerly_complete")]
        eagerly_complete: bool,

        #[serde(default = "default_eagerly_fail")]
        eagerly_fail: bool,

        schema: serde_json::Value,
        inlets_count: usize,
    },
}

fn default_eagerly_complete() -> bool {
    false
}

fn default_eagerly_fail() -> bool {
    false
}
