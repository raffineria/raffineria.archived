use crate::protocol::Schema;

use super::schema_util::record_schema;

lazy_static! {
    pub static ref PORT_PUSH_SCHEMA: Schema = record_schema(
        "PortPush",
        vec![(
            "items",
            Schema::Array(Box::new(Schema::Array(Box::new(Schema::Int))))
        )]
    );
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PortPush {
    pub items: Vec<Vec<u8>>,
}

#[test]
fn serde_test() {
    super::serde_test_util::run_serde(
        PortPush {
            items: vec![vec![1, 2, 3]],
        },
        &*PORT_PUSH_SCHEMA,
    )
    .unwrap();
}
