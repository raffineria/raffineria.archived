use super::*;

#[derive(Fail, Debug)]
pub enum RunnerError {
    #[fail(display = "RunnerError::SchemaError")]
    GraphDefinitionError(#[cause] GraphDefinitionError),

    #[fail(display = "RunnerError::OsProcessError")]
    OsProcessError(#[cause] os_process::OsProcessError),

    #[fail(display = "RunnerError::Generic")]
    Generic(#[cause] failure::Error),

    #[fail(display = "RunnerError::NotImplemented")]
    NotImplemented,
}

impl From<GraphDefinitionError> for RunnerError {
    fn from(inner: GraphDefinitionError) -> Self {
        RunnerError::GraphDefinitionError(inner)
    }
}

impl From<os_process::OsProcessError> for RunnerError {
    fn from(inner: os_process::OsProcessError) -> Self {
        RunnerError::OsProcessError(inner)
    }
}
