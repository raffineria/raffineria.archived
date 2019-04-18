use futures::prelude::*;

use crate::futures::fsm::FSM;

use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};
use crate::protocol::Schema;

use super::*;

impl StdStage for Tee {
    fn inlet_schemas(&self) -> Vec<&Schema> {
        vec![&self.schema]
    }

    fn outlet_schemas(&self) -> Vec<&Schema> {
        (0..self.outlets_count).map(|_| &self.schema).collect()
    }

    fn run_with_channels(
        self: Box<Self>,
        mut inlets: Vec<(ConsumerTx, ConsumerRx)>,
        outlets: Vec<(ProducerTx, ProducerRx)>,
    ) -> RunningFuture {
        assert!(inlets.len() == 1);
        let inlet = inlets.pop().unwrap();

        Box::new(
            TeeFSM::new(inlet, outlets)
                .into_fsm_future()
                .map_err(|err| StdStageError::Generic(err.into())),
        )
    }
}
