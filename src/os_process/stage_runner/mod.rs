use crate::futures::fsm::FSMFuture;
use crate::protocol::streams::{OwnStdinInlet, OwnStdoutOutlet};

use super::Stage;

mod stage_failure;
pub use stage_failure::StageFailure;

mod substates;

mod stage_runner_impl;
mod stage_runner_impl_fsm;

pub enum StageRunner<S: Stage> {
    Init {
        stage: S,

        protocol_in: OwnStdinInlet,
        protocol_out: OwnStdoutOutlet,
    },
    Handshake {
        protocol_in: OwnStdinInlet,
        handshake: FSMFuture<substates::Handshake<S>>,
    },

    Running {
        running: substates::Running,
    },
}
