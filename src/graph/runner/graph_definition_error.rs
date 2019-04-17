#[derive(Fail, Debug)]
pub enum GraphDefinitionError {
    #[fail(display = "GraphDefinitionError::SchemaParseError")]
    SchemaParseError(#[cause] failure::Error),

    #[fail(display = "GraphDefinitionError::DuplicateInletName: {}", _0)]
    DuplicateInletName(String),

    #[fail(display = "GraphDefinitionError::DuplicateOutletName: {}", _0)]
    DuplicateOutletName(String),

    #[fail(display = "GraphDefinitionError::VertexDoesNotExist: {}", _0)]
    VertexDoesNotExist(String),

    #[fail(display = "GraphDefinitionError::PortDoesNotExist: {}", _0)]
    PortDoesNotExist(String),

    #[fail(display = "GraphDefinitionError::ExternalPortMismatch")]
    ExternalPortMismatch,

    #[fail(
        display = "GraphDefinitionError::UnboundPorts [vertex: {}; outlets: {:?}; inlets: {:?}]",
        vertex, outlets, inlets
    )]
    UnboundPorts {
        vertex: String,
        outlets: Vec<String>,
        inlets: Vec<String>,
    },

    #[fail(display = "GraphDefinitionError::Generic")]
    Generic(#[cause] failure::Error),
}
