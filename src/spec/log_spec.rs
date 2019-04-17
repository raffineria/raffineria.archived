#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "log_spec")]
pub enum LogSpec {
    #[serde(rename = "null")]
    Null,

    #[serde(rename = "no_capture")]
    NoCapture,

    #[serde(rename = "file")]
    File { path: String },
}

impl Default for LogSpec {
    fn default() -> Self {
        LogSpec::NoCapture
    }
}
