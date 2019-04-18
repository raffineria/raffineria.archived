use futures::prelude::*;

use crate::futures::fsm::FSM;
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};
use crate::protocol::Schema;

use super::*;

impl StdStage for Merge {
    fn inlet_schemas(&self) -> Vec<&Schema> {
        (0..self.inlets_count).map(|_| &self.schema).collect()
    }

    fn outlet_schemas(&self) -> Vec<&Schema> {
        vec![&self.schema]
    }

    fn run_with_channels(
        self: Box<Self>,
        inlets: Vec<(ConsumerTx, ConsumerRx)>,
        mut outlets: Vec<(ProducerTx, ProducerRx)>,
    ) -> RunningFuture {
        assert!(outlets.len() == 1);
        let outlet = outlets.pop().unwrap();

        Box::new(
            MergeFSM::new(inlets, outlet, self.eagerly_complete, self.eagerly_fail)
                .into_fsm_future()
                .map_err(|err| StdStageError::Generic(err.into())),
        )
    }
}
