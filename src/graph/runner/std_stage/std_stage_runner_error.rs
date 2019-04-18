use crate::protocol::SchemaResolutionError;
use crate::std_stages::StdStageError;

#[derive(Fail, Debug)]
pub enum StdStageRunnerError {
    #[fail(display = "StdStageRunnerError::StdStageError")]
    SpecResolveError(#[cause] StdStageError),

    #[fail(display = "StdStageRunnerError::SchemaResolutionError")]
    SchemaResolutionError(#[cause] SchemaResolutionError),

    #[fail(
        display = "StdStageRunnerError::PortCountMismatch [expected-outlets: {}; actual-outlets: {}; expected-inlets: {}; actual-inlets: {}]",
        expected_outlets, actual_outlets, expected_inlets, actual_inlets
    )]
    PortCountMismatch {
        expected_inlets: usize,
        actual_inlets: usize,
        expected_outlets: usize,
        actual_outlets: usize,
    },

    #[fail(display = "StdStageRunnerError::MpscError")]
    MpscError,

    #[fail(display = "StdStageRunnerError::RunningError")]
    RunningError(#[cause] StdStageError),

    #[fail(display = "StdStageRunnerError::Generic")]
    Generic(#[cause] failure::Error),
}
