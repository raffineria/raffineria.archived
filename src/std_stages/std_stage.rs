use futures::sync::mpsc;
use std::fmt;

use crate::futures::SendBoxedFuture;
use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};
use crate::protocol::Schema;
use crate::spec::StdStageSpec;

use super::*;

pub type BoxedStdStage = Box<dyn StdStage>;
pub type RunningFuture = SendBoxedFuture<(), StdStageError>;

pub trait StdStage: Send + fmt::Debug {
    fn inlet_schemas(&self) -> Vec<&Schema>;

    fn outlet_schemas(&self) -> Vec<&Schema>;

    fn run_with_channels(
        self: Box<Self>,
        inlets: Vec<(ConsumerTx, ConsumerRx)>,
        outlets: Vec<(ProducerTx, ProducerRx)>,
    ) -> RunningFuture;

    fn into_running(
        self: Box<Self>,
    ) -> (
        RunningFuture,
        Vec<(ProducerTx, ProducerRx)>,
        Vec<(ConsumerTx, ConsumerRx)>,
    ) {
        let (inlet_chans_outside, inlet_chans_inside): (Vec<_>, Vec<_>) =
            (0..self.inlet_schemas().len())
                .map(|_| {
                    let (producer_tx, consumer_rx) = mpsc::channel::<ProducerMessage>(0);
                    let (consumer_tx, producer_rx) = mpsc::channel::<ConsumerMessage>(0);

                    ((producer_tx, producer_rx), (consumer_tx, consumer_rx))
                })
                .unzip();

        let (outlet_chans_outside, outlet_chans_inside): (Vec<_>, Vec<_>) =
            (0..self.outlet_schemas().len())
                .map(|_| {
                    let (producer_tx, consumer_rx) = mpsc::channel::<ProducerMessage>(0);
                    let (consumer_tx, producer_rx) = mpsc::channel::<ConsumerMessage>(0);

                    ((consumer_tx, consumer_rx), (producer_tx, producer_rx))
                })
                .unzip();

        (
            self.run_with_channels(inlet_chans_inside, outlet_chans_inside),
            inlet_chans_outside,
            outlet_chans_outside,
        )
    }
}

pub fn from_spec(spec: StdStageSpec) -> Result<BoxedStdStage, StdStageError> {
    match spec {
        StdStageSpec::Tee {
            schema,
            outlets_count,
        } => Ok(Box::new(Tee::new(parse_schema(schema)?, outlets_count))),

        StdStageSpec::Merge {
            schema,
            inlets_count,
            eagerly_complete,
            eagerly_fail,
        } => Ok(Box::new(Merge::new(
            parse_schema(schema)?,
            inlets_count,
            eagerly_complete,
            eagerly_fail,
        ))),
    }
}

fn parse_schema(json: serde_json::Value) -> Result<Schema, StdStageError> {
    Schema::parse(&json).map_err(|err| StdStageError::SchemaParseError(err))
}
