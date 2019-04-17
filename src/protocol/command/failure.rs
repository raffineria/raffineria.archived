use crate::protocol::Schema;

use super::schema_util::record_schema;

lazy_static! {
    pub static ref FAILURE_SCHEMA: Schema = record_schema(
        "Failure",
        vec![
            ("message", Schema::String),
            (
                "reason_chain",
                Schema::Array(Box::new(super::FAILURE_REASON_SCHEMA.clone()))
            )
        ]
    );
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Failure {
    pub message: String,
    pub reason_chain: Vec<super::FailureReason>,
}

impl From<failure::Error> for Failure {
    fn from(f: failure::Error) -> Self {
        let message = format!("{:?}", f);
        let reason_chain = vec![];
        Failure {
            message,
            reason_chain,
        }
    }
}
