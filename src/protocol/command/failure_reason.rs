use crate::protocol::Schema;

use super::schema_util::record_schema;

lazy_static! {
    pub static ref FAILURE_REASON_SCHEMA: Schema =
        record_schema("FailureReason", vec![("message", Schema::String),]);
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FailureReason {
    pub message: String,
}
