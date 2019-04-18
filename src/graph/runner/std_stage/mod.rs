use crate::futures::fsm::FSMFuture;

use super::*;

mod std_stage_runner_error;
pub use std_stage_runner_error::StdStageRunnerError;

mod std_stage_runner;
mod std_stage_runner_impl_fsm;
pub use std_stage_runner::StdStageRunner;

pub type StdStageRunnerFuture = FSMFuture<StdStageRunner>;
