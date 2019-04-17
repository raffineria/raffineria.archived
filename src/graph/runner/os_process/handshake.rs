use std::collections::VecDeque;

use boxfnonce::SendBoxFnOnce;
use futures::prelude::*;

use crate::futures::fsm::*;
use crate::protocol::streams::{ChildStdinOutlet, ChildStdoutInlet};
use crate::protocol::{Command, Schema, SchemaResolutionError};

use super::*;

#[derive(Fail, Debug)]
pub enum HandshakeError {
    #[fail(display = "HandshakeError::UnexpectedCommand: {:?}", _0)]
    UnexpectedCommand(Command),

    #[fail(display = "HandshakeError::ProtocolInletError")]
    ProtocolInletError(#[cause] failure::Error),

    #[fail(display = "HandshakeError::ProtocolInletTerminated")]
    ProtocolInletTerminated,

    #[fail(display = "HandshakeError::SchemaParseError")]
    SchemaParseError(#[cause] failure::Error),

    #[fail(display = "HandshakeError::SchemaResolutionError")]
    SchemaResolutionError(#[cause] SchemaResolutionError),

    #[fail(
        display = "HandshakeError::PortCountMismatch [expected-outlets: {}; actual-outlets: {}; expected-inlets: {}; actual-inlets: {}]",
        expected_outlets, actual_outlets, expected_inlets, actual_inlets
    )]
    PortCountMismatch {
        expected_inlets: usize,
        actual_inlets: usize,
        expected_outlets: usize,
        actual_outlets: usize,
    },

    #[fail(display = "HandshakeError::Generic")]
    Generic(#[cause] failure::Error),

    #[fail(display = "HandshakeError::NotImplemented")]
    NotImplemented,
}

pub struct HandshakeDone {
    pub process_handle: spawn::ProcessHandle,
    pub protocol_inlet: ChildStdoutInlet,
    pub protocol_outlet: ChildStdinOutlet,
    pub inlets_with_resolution: Vec<graph_channels::ConsumerChannelsWithResolution>,
    pub outlets_with_resolution: Vec<graph_channels::ProducerChannelsWithResolution>,
}

pub enum Handshake {
    Init {
        process_handle: spawn::ProcessHandle,
        protocol_inlet: ChildStdoutInlet,
        protocol_outlet: ChildStdinOutlet,
        inlets: Vec<graph_channels::ConsumerChannels>,
        outlets: Vec<graph_channels::ProducerChannels>,
    },

    CollectingSchemas {
        process_handle: spawn::ProcessHandle,
        schemas: VecDeque<Schema>,
        outlets_count: usize,
        inlets_count: usize,
        protocol_inlet: ChildStdoutInlet,
        protocol_outlet: ChildStdinOutlet,
        inlets: Vec<graph_channels::ConsumerChannels>,
        outlets: Vec<graph_channels::ProducerChannels>,
    },

    Receiving {
        protocol_inlet: ChildStdoutInlet,
        and_then: SendBoxFnOnce<'static, (Command, ChildStdoutInlet), TurnResult<Handshake>>,
    },
}

impl FSM for Handshake {
    type Item = HandshakeDone;
    type Error = HandshakeError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            Handshake::Receiving {
                mut protocol_inlet,
                and_then,
            } => protocol_inlet
                .poll()
                .map_err(|err| HandshakeError::ProtocolInletError(err))
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(Handshake::Receiving {
                        protocol_inlet,
                        and_then,
                    })),
                    Async::Ready(None) => Err(HandshakeError::ProtocolInletTerminated),
                    Async::Ready(Some(command)) => and_then.call(command, protocol_inlet),
                }),

            Handshake::Init {
                process_handle,
                protocol_inlet,
                protocol_outlet,
                inlets,
                outlets,
            } => Ok(TurnOk::PollMore(Handshake::Receiving {
                protocol_inlet,
                and_then: SendBoxFnOnce::from(move |command, protocol_inlet| match command {
                    Command::Hello {
                        version: _,
                        inlets_count,
                        outlets_count,
                    } => {
                        let inlets_count = inlets_count as usize;
                        let outlets_count = outlets_count as usize;

                        if inlets_count != inlets.len() || outlets_count != outlets.len() {
                            Err(HandshakeError::PortCountMismatch {
                                expected_outlets: outlets.len(),
                                actual_outlets: outlets_count,
                                expected_inlets: inlets.len(),
                                actual_inlets: inlets_count,
                            })
                        } else {
                            Ok(TurnOk::PollMore(Handshake::CollectingSchemas {
                                process_handle,
                                schemas: VecDeque::new(),
                                inlets_count,
                                outlets_count,
                                protocol_inlet,
                                protocol_outlet,
                                outlets,
                                inlets,
                            }))
                        }
                    }

                    command => Err(HandshakeError::UnexpectedCommand(command)),
                }),
            })),

            Handshake::CollectingSchemas {
                process_handle,
                mut schemas,
                inlets_count,
                outlets_count,
                protocol_inlet,
                protocol_outlet,
                outlets,
                inlets,
            } => {
                if schemas.len() < inlets_count + outlets_count {
                    Ok(TurnOk::PollMore(Handshake::Receiving {
                        protocol_inlet,
                        and_then: SendBoxFnOnce::from(
                            move |command, protocol_inlet| match command {
                                Command::PortDeclare { schema } => Schema::parse_str(&schema)
                                    .map(move |schema| {
                                        schemas.push_back(schema);
                                        TurnOk::PollMore(Handshake::CollectingSchemas {
                                            process_handle,
                                            schemas,
                                            inlets_count,
                                            outlets_count,
                                            protocol_inlet,
                                            protocol_outlet,
                                            outlets,
                                            inlets,
                                        })
                                    })
                                    .map_err(|schema_parse_err| {
                                        HandshakeError::SchemaParseError(schema_parse_err)
                                    }),

                                command => Err(HandshakeError::UnexpectedCommand(command)),
                            },
                        ),
                    }))
                } else {
                    let schemas = schemas.into_iter().collect::<Vec<_>>();

                    let outlet_schemas = schemas[..outlets_count].iter();
                    let inlet_schemas = schemas[outlets_count..].iter();

                    let outlets_with_resolution = outlet_schemas
                        .zip(outlets.into_iter())
                        .map(|(writer_schema, outlet_chans)| {
                            outlet_chans.resolve_with_writer_schema(writer_schema)
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|schema_resolution_err| {
                            HandshakeError::SchemaResolutionError(schema_resolution_err)
                        })?;

                    let inlets_with_resolution = inlet_schemas
                        .zip(inlets.into_iter())
                        .map(|(reader_schema, inlet_chans)| {
                            inlet_chans.resolve_with_reader_schema(reader_schema)
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|schema_resolution_err| {
                            HandshakeError::SchemaResolutionError(schema_resolution_err)
                        })?;

                    trace!(
                        "Handshake.Ready: outlets: {:?}; inlets: {:?}",
                        outlets_with_resolution,
                        inlets_with_resolution
                    );
                    Ok(TurnOk::Ready(HandshakeDone {
                        process_handle,
                        protocol_inlet,
                        protocol_outlet,
                        inlets_with_resolution,
                        outlets_with_resolution,
                    }))
                }
            }
        }
    }
}

pub type HandshakeDoneFuture = FSMFuture<Handshake>;

pub fn handshake(
    process_handle: spawn::ProcessHandle,
    protocol_inlet: ChildStdoutInlet,
    protocol_outlet: ChildStdinOutlet,
    inlets: Vec<graph_channels::ConsumerChannels>,
    outlets: Vec<graph_channels::ProducerChannels>,
) -> HandshakeDoneFuture {
    let initial_state = Handshake::Init {
        process_handle,
        protocol_inlet,
        protocol_outlet,
        inlets,
        outlets,
    };
    initial_state.into_fsm_future()
}
