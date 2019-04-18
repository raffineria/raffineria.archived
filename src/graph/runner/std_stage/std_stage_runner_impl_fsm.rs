use futures::future;
use futures::prelude::*;

use crate::futures::fsm::*;

use crate::spec::StdStageSpec;
use crate::std_stages::BoxedStdStage;

use graph_channels::{ConsumerChannels, ConsumerChannelsWithResolution};
use graph_channels::{ProducerChannels, ProducerChannelsWithResolution};

use super::*;

impl FSM for StdStageRunner {
    type Item = ();
    type Error = StdStageRunnerError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            StdStageRunner::Init {
                spec,
                inlets,
                outlets,
            } => resolve_spec(spec, inlets, outlets),

            StdStageRunner::SpecResolved {
                stage,
                inlets,
                outlets,
            } => resolve_schemas(stage, inlets, outlets),

            StdStageRunner::SchemasResolved {
                stage,
                inlets,
                outlets,
            } => wire_up(stage, inlets, outlets),

            StdStageRunner::Running { mut inner } => inner.poll().map(|poll| match poll {
                Async::NotReady => TurnOk::Suspend(StdStageRunner::Running { inner }),
                Async::Ready(()) => TurnOk::Ready(()),
            }),
        }
    }
}

fn resolve_spec(
    spec: StdStageSpec,
    inlets: Vec<ConsumerChannels>,
    outlets: Vec<ProducerChannels>,
) -> TurnResult<StdStageRunner> {
    crate::std_stages::from_spec(spec)
        .map_err(|err| StdStageRunnerError::SpecResolveError(err))
        .map(|stage| {
            TurnOk::PollMore(StdStageRunner::SpecResolved {
                stage,
                inlets,
                outlets,
            })
        })
}

fn resolve_schemas(
    stage: BoxedStdStage,
    inlets: Vec<ConsumerChannels>,
    outlets: Vec<ProducerChannels>,
) -> TurnResult<StdStageRunner> {
    let inlet_schemas = stage.inlet_schemas();
    let outlet_schemas = stage.outlet_schemas();

    let inlets = inlets
        .into_iter()
        .zip(inlet_schemas.into_iter())
        .map(|(chans, reader_schema)| chans.resolve_with_reader_schema(reader_schema))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| StdStageRunnerError::SchemaResolutionError(err))?;

    let outlets = outlets
        .into_iter()
        .zip(outlet_schemas.into_iter())
        .map(|(chans, writer_schema)| chans.resolve_with_writer_schema(writer_schema))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| StdStageRunnerError::SchemaResolutionError(err))?;

    Ok(TurnOk::PollMore(StdStageRunner::SchemasResolved {
        stage,
        inlets,
        outlets,
    }))
}

fn wire_up(
    stage: BoxedStdStage,
    inlets_peer: Vec<ConsumerChannelsWithResolution>,
    outlets_peer: Vec<ProducerChannelsWithResolution>,
) -> TurnResult<StdStageRunner> {
    let (running_fut, inlets_stage, outlets_stage) = stage.into_running();

    if inlets_peer.len() != inlets_stage.len() || outlets_peer.len() != outlets_stage.len() {
        Err(StdStageRunnerError::PortCountMismatch {
            expected_inlets: inlets_peer.len(),
            actual_inlets: inlets_stage.len(),
            expected_outlets: outlets_peer.len(),
            actual_outlets: outlets_stage.len(),
        })
    } else {
        let inlets_wired_up =
            future::join_all(inlets_peer.into_iter().zip(inlets_stage.into_iter()).map(
                |(inlet_peer, (tx, rx))| {
                    let into_stage = rx.map_err(|_| StdStageRunnerError::MpscError).forward(
                        inlet_peer
                            .tx
                            .sink_map_err(|_| StdStageRunnerError::MpscError),
                    );

                    let from_stage = inlet_peer
                        .rx
                        .map_err(|_| StdStageRunnerError::MpscError)
                        .forward(tx.sink_map_err(|_| StdStageRunnerError::MpscError));

                    into_stage.join(from_stage)
                },
            ));

        let outlets_wired_up =
            future::join_all(outlets_peer.into_iter().zip(outlets_stage.into_iter()).map(
                |(outlets_peer, (tx, rx))| {
                    let into_stage = rx.map_err(|_| StdStageRunnerError::MpscError).forward(
                        outlets_peer
                            .tx
                            .sink_map_err(|_| StdStageRunnerError::MpscError),
                    );

                    let from_stage = outlets_peer
                        .rx
                        .map_err(|_| StdStageRunnerError::MpscError)
                        .forward(tx.sink_map_err(|_| StdStageRunnerError::MpscError));

                    into_stage.join(from_stage)
                },
            ));

        let inner = Box::new(
            inlets_wired_up
                .join(outlets_wired_up)
                .join(running_fut.map_err(|err| StdStageRunnerError::RunningError(err)))
                .map(|(_, ())| ()),
        );

        Ok(TurnOk::PollMore(StdStageRunner::Running { inner }))
    }
}
