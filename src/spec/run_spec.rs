use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "run_spec")]
pub enum RunSpec {
    #[serde(rename = "os_process")]
    OsProcess {
        cmd: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        log: LogSpec,
    },

    #[serde(rename = "graph")]
    Graph(Box<GraphSpec>),

    #[serde(rename = "std")]
    StdStage(StdStageSpec),
}
