use crate::futures::fsm::*;
use futures::prelude::*;

use super::substates;
use super::{Stage, StageFailure, StageRunner};

impl<S: Stage> FSM for StageRunner<S> {
    type Item = ();
    type Error = StageFailure;

    fn turn(self) -> TurnResult<Self> {
        match self {
            StageRunner::Init {
                protocol_in,
                protocol_out,
                stage,
            } => {
                let handshake = substates::Handshake::new(stage, protocol_out).into_fsm_future();
                Ok(TurnOk::PollMore(StageRunner::Handshake {
                    protocol_in,
                    handshake,
                }))
            }

            StageRunner::Handshake {
                protocol_in,
                mut handshake,
            } => handshake.poll().map_err(|reason| reason.into()).map(
                |async_poll| match async_poll {
                    Async::NotReady => TurnOk::Suspend(StageRunner::Handshake {
                        protocol_in,
                        handshake,
                    }),
                    Async::Ready((stage, protocol_out)) => TurnOk::PollMore(StageRunner::Running {
                        running: substates::Running::new(stage, protocol_out, protocol_in),
                    }),
                },
            ),

            StageRunner::Running { mut running } => running
                .poll()
                .map_err(|reason| reason.into())
                .map(|poll| match poll {
                    Async::NotReady => TurnOk::Suspend(StageRunner::Running { running }),
                    Async::Ready((_, _)) => TurnOk::Ready(()),
                }),
        }
    }
}
