use crate::futures::fsm::{FSMFuture, FSM};
use crate::futures::{SendBoxedSink, SendBoxedStream};
use crate::protocol::{DataItem, Schema};

use super::{Ports, StageRunner};

lazy_static! {
    static ref SCHEMAS_EMPTY_VEC: Vec<Schema> = vec![];
}

pub trait Stage: Sized {
    fn outlets(&self) -> &Vec<Schema> {
        &*SCHEMAS_EMPTY_VEC
    }
    fn inlets(&self) -> &Vec<Schema> {
        &*SCHEMAS_EMPTY_VEC
    }

    fn into_future(self, ports: Ports) -> FSMFuture<StageRunner<Self>> {
        StageRunner::new(self, ports.protocol_in, ports.protocol_out).into_fsm_future()
    }

    fn into_streams(
        self,
    ) -> (
        Vec<SendBoxedStream<DataItem, failure::Error>>,
        Vec<SendBoxedSink<DataItem, failure::Error>>,
    );
}
