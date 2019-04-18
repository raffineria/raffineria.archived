#[derive(Fail, Debug)]
pub enum StdStageError {
    #[fail(display = "StdStageError::SchemaParseError")]
    SchemaParseError(#[cause] failure::Error),

    #[fail(display = "StdStageError::Generic")]
    Generic(#[cause] failure::Error),
}
