use crate::protocol::Schema;

use super::schema_util::record_schema;

lazy_static! {
    pub static ref PORT_PULL_SCHEMA: Schema =
        record_schema("PortPull", vec![("max_items", Schema::Int),]);
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PortPull {
    pub max_items: i32,
}
