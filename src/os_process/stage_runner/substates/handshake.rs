use futures::prelude::*;

use crate::futures::fsm::*;
use crate::futures::SendBoxedFuture;
use crate::protocol::streams::OwnStdoutOutlet;
use crate::protocol::Command;

use crate::os_process::Stage;

#[derive(Fail, Debug)]
pub enum HandshakeFailure {
    #[fail(display = "HandshakeFailure::OwnStdoutOutletFailure")]
    OwnStdoutOutletFailure(#[cause] failure::Error),
}

pub enum Handshake<S: Stage> {
    Init {
        stage: S,
        protocol_out: OwnStdoutOutlet,
    },
    SendingCommands {
        stage: S,
        commands_sent: SendBoxedFuture<OwnStdoutOutlet, failure::Error>,
    },
}

impl<S: Stage> Handshake<S> {
    pub fn new(stage: S, protocol_out: OwnStdoutOutlet) -> Self {
        Handshake::Init {
            stage,
            protocol_out,
        }
    }
}

impl<S: Stage> FSM for Handshake<S> {
    type Item = (S, OwnStdoutOutlet);
    type Error = HandshakeFailure;

    fn turn(self) -> TurnResult<Self> {
        match self {
            Handshake::Init {
                stage,
                protocol_out,
            } => {
                let hello = Command::Hello {
                    version: 0,
                    inlets_count: stage.inlets().len() as i32,
                    outlets_count: stage.outlets().len() as i32,
                };
                let outlet_declarations =
                    stage.outlets().iter().map(|schema| Command::PortDeclare {
                        schema: schema.canonical_form(),
                    });
                let inlet_declarations = stage.inlets().iter().map(|schema| Command::PortDeclare {
                    schema: schema.canonical_form(),
                });

                let port_declarations = outlet_declarations.chain(inlet_declarations);

                let hello_sent = Box::new(protocol_out.send(hello));
                let commands_sent = port_declarations.fold::<SendBoxedFuture<_, _>, _>(
                    hello_sent,
                    |prev_sent, command| {
                        Box::new(prev_sent.and_then(|protocol_out| protocol_out.send(command)))
                    },
                );

                Ok(TurnOk::PollMore(Handshake::SendingCommands {
                    stage,
                    commands_sent,
                }))
            }

            Handshake::SendingCommands {
                stage,
                mut commands_sent,
            } => commands_sent
                .poll()
                .map_err(|reason| HandshakeFailure::OwnStdoutOutletFailure(reason))
                .map(|async_poll| match async_poll {
                    Async::NotReady => TurnOk::Suspend(Handshake::SendingCommands {
                        stage,
                        commands_sent,
                    }),
                    Async::Ready(protocol_out) => TurnOk::Ready((stage, protocol_out)),
                }),
        }
    }
}
