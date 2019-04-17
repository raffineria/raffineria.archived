use super::schema_util::record_schema;
use crate::protocol::Schema;
use avro_rs::schema::UnionSchema;

use super::{Failure, PortPull, PortPush};

lazy_static! {
    static ref HELLO_SCHEMA: Schema = record_schema(
        "hello",
        vec![
            ("version", Schema::Int),
            ("outlets_count", Schema::Int),
            ("inlets_count", Schema::Int)
        ]
    );
    static ref PORT_DECLARE_SCHEMA: Schema =
        record_schema("port_declare", vec![("schema", Schema::String),]);
    static ref PORT_PULL_SCHEMA: Schema = record_schema(
        "port_pull",
        vec![
            ("port_id", Schema::Int),
            ("inner", super::PORT_PULL_SCHEMA.clone())
        ]
    );
    static ref PORT_PUSH_SCHEMA: Schema = record_schema(
        "port_push",
        vec![
            ("port_id", Schema::Int),
            ("inner", super::PORT_PUSH_SCHEMA.clone())
        ]
    );
    static ref OUTLET_COMPLETED_SCHEMA: Schema =
        record_schema("outlet_completed", vec![("port_id", Schema::Int),]);
    static ref OUTLET_FAILED_SCHEMA: Schema = record_schema(
        "outlet_failed",
        vec![
            ("port_id", Schema::Int),
            ("inner", super::FAILURE_SCHEMA.clone())
        ]
    );
    static ref INLET_CANCELLED_SCHEMA: Schema =
        record_schema("inlet_cancelled", vec![("port_id", Schema::Int),]);
    pub static ref COMMAND_SCHEMA: Schema = Schema::Union(
        UnionSchema::new(vec![
            HELLO_SCHEMA.clone(),
            PORT_DECLARE_SCHEMA.clone(),
            PORT_PULL_SCHEMA.clone(),
            PORT_PUSH_SCHEMA.clone(),
            OUTLET_COMPLETED_SCHEMA.clone(),
            OUTLET_FAILED_SCHEMA.clone(),
            INLET_CANCELLED_SCHEMA.clone(),
        ])
        .unwrap()
    );
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Command {
    #[serde(rename = "hello")]
    Hello {
        version: i32,
        outlets_count: i32,
        inlets_count: i32,
    },

    #[serde(rename = "port_declare")]
    PortDeclare { schema: String },

    #[serde(rename = "port_pull")]
    PortPull { port_id: i32, inner: PortPull },

    #[serde(rename = "port_push")]
    PortPush { port_id: i32, inner: PortPush },

    #[serde(rename = "outlet_completed")]
    OutletCompleted { port_id: i32 },

    #[serde(rename = "outlet_failed")]
    OutletFailed { port_id: i32, inner: Failure },

    #[serde(rename = "inlet_cancelled")]
    InletCancelled { port_id: i32 },
}

impl Command {
    pub fn schema() -> &'static Schema {
        &COMMAND_SCHEMA
    }

    pub fn outlet_idx(&self) -> Option<usize> {
        let port_id_opt = match *self {
            Command::PortPull { port_id, .. } => Some(port_id),
            Command::InletCancelled { port_id, .. } => Some(port_id),
            _ => None,
        };
        port_id_opt.map(|port_id| port_id as usize)
    }

    pub fn inlet_idx(&self) -> Option<usize> {
        let port_id_opt = match *self {
            Command::PortPush { port_id, .. } => Some(port_id),
            Command::OutletCompleted { port_id, .. } => Some(port_id),
            Command::OutletFailed { port_id, .. } => Some(port_id),
            _ => None,
        };
        port_id_opt.map(|port_id| port_id as usize)
    }
}

#[test]
fn serde_test() {
    use super::FailureReason;

    let commands = vec![
        Command::Hello {
            version: 0,
            outlets_count: 2,
            inlets_count: 2,
        },
        Command::PortDeclare {
            schema: r#"{"type": "string"}"#.to_owned(),
        },
        Command::PortPull {
            port_id: 1,
            inner: PortPull { max_items: 5 },
        },
        Command::PortPush {
            port_id: 1,
            inner: PortPush {
                items: vec![vec![1, 2, 3]],
            },
        },
        Command::OutletCompleted { port_id: 1 },
        Command::OutletFailed {
            port_id: 1,
            inner: Failure {
                message: "abc".to_owned(),
                reason_chain: vec![
                    FailureReason {
                        message: "one".to_owned(),
                    },
                    FailureReason {
                        message: "two".to_owned(),
                    },
                    FailureReason {
                        message: "three".to_owned(),
                    },
                ],
            },
        },
        Command::InletCancelled { port_id: 2 },
    ];
    for command in commands.iter() {
        super::serde_test_util::run_serde(command.clone(), &*COMMAND_SCHEMA)
            .expect(&format!("Failed to run_serde with {:?}", command));
    }
}
