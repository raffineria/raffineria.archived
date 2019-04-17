#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "restart_strategy")]
pub enum RestartStrategySpec {
    #[serde(rename = "no_restart")]
    NoRestart,
}

impl Default for RestartStrategySpec {
    fn default() -> Self {
        RestartStrategySpec::NoRestart
    }
}
