use futures::prelude::*;
use futures::sync::mpsc;
use std::collections::HashSet;

use crate::protocol::command::{PortPull, PortPush};
use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::Command;

use super::*;
use message_channels::{ConsumerTx, ProducerTx};

// type CommandsSink = SendBoxedSink<Command, CommandToMessageError>;

#[derive(Fail, Debug)]
pub enum CommandToMessageError {
    #[fail(display = "CommandToMessageError::ProtocolInletFailure")]
    ProtocolInletFailure(#[cause] failure::Error),

    #[fail(display = "CommandToMessageError::UnexpectedCommand: {:?}", command)]
    UnexpectedCommand { command: Command },

    #[fail(display = "CommandToMessageError::MpscProducerTxError")]
    MpscProducerTxError(#[cause] mpsc::SendError<ProducerMessage>),

    #[fail(display = "CommandToMessageError::MpscConsumerTxError")]
    MpscConsumerTxError(#[cause] mpsc::SendError<ConsumerMessage>),

    #[fail(display = "CommandToMessageError::Generic")]
    Generic(#[cause] failure::Error),
}

impl From<mpsc::SendError<ProducerMessage>> for CommandToMessageError {
    fn from(sender_error: mpsc::SendError<ProducerMessage>) -> Self {
        CommandToMessageError::MpscProducerTxError(sender_error)
    }
}
impl From<mpsc::SendError<ConsumerMessage>> for CommandToMessageError {
    fn from(sender_error: mpsc::SendError<ConsumerMessage>) -> Self {
        CommandToMessageError::MpscConsumerTxError(sender_error)
    }
}

pub struct CommandToMessage {
    producer_txs: Vec<ProducerTx>,
    producers_to_poll: HashSet<usize>,
    consumer_txs: Vec<ConsumerTx>,
    consumers_to_poll: HashSet<usize>,
}

impl CommandToMessage {
    pub fn new(producer_txs: Vec<ProducerTx>, consumer_txs: Vec<ConsumerTx>) -> Self {
        let producers_to_poll = HashSet::with_capacity(producer_txs.len());
        let consumers_to_poll = HashSet::with_capacity(consumer_txs.len());
        Self {
            producer_txs,
            producers_to_poll,
            consumer_txs,
            consumers_to_poll,
        }
    }
}

impl Sink for CommandToMessage {
    type SinkItem = Command;
    type SinkError = CommandToMessageError;

    fn start_send(
        &mut self,
        command: Self::SinkItem,
    ) -> StartSend<Self::SinkItem, Self::SinkError> {
        trace!(
            "<CommandToMessage as Sink>::start_send(self, {:?})",
            command
        );

        match command_to_message(&command) {
            (Some((inlet_idx, producer_message)), None) => self
                .producer_txs
                .get_mut(inlet_idx)
                .expect("producer_txs broken")
                .start_send(producer_message)
                .map_err(|send_err| send_err.into())
                .map(|sink| match sink {
                    AsyncSink::NotReady(_) => AsyncSink::NotReady(command),
                    AsyncSink::Ready => {
                        self.producers_to_poll.insert(inlet_idx);
                        AsyncSink::Ready
                    }
                }),

            (None, Some((outlet_idx, consumer_message))) => self
                .consumer_txs
                .get_mut(outlet_idx)
                .expect("consumer_txs broken")
                .start_send(consumer_message)
                .map_err(|send_err| send_err.into())
                .map(|sink| match sink {
                    AsyncSink::NotReady(_) => AsyncSink::NotReady(command),
                    AsyncSink::Ready => {
                        self.consumers_to_poll.insert(outlet_idx);
                        AsyncSink::Ready
                    }
                }),

            (_, _) => Err(CommandToMessageError::UnexpectedCommand { command }),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let consumer_polls = self
            .consumers_to_poll
            .clone()
            .into_iter()
            .map(|outlet_idx| {
                self.consumer_txs
                    .get_mut(outlet_idx)
                    .expect("consumer_txs broken")
                    .poll_complete()
                    .map(|poll| (outlet_idx, poll))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let conumers_to_poll = consumer_polls
            .into_iter()
            .filter_map(|(outlet_idx, poll)| match poll {
                Async::NotReady => Some(outlet_idx),
                Async::Ready(()) => None,
            })
            .collect::<HashSet<_>>();

        let producer_polls = self
            .producers_to_poll
            .clone()
            .into_iter()
            .map(|inlet_idx| {
                self.producer_txs
                    .get_mut(inlet_idx)
                    .expect("producer_txs broken")
                    .poll_complete()
                    .map(|poll| (inlet_idx, poll))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let producers_to_poll = producer_polls
            .into_iter()
            .filter_map(|(inlet_idx, poll)| match poll {
                Async::NotReady => Some(inlet_idx),
                Async::Ready(()) => None,
            })
            .collect::<HashSet<_>>();

        self.producers_to_poll = producers_to_poll;
        self.consumers_to_poll = conumers_to_poll;

        if self.producers_to_poll.is_empty() && self.consumers_to_poll.is_empty() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn command_to_message(
    command: &Command,
) -> (
    Option<(usize, ProducerMessage)>,
    Option<(usize, ConsumerMessage)>,
) {
    match *command {
        Command::PortPull {
            port_id,
            inner: PortPull { max_items },
        } => (
            None,
            Some((
                port_id as usize,
                ConsumerMessage::Pull {
                    max_items: max_items as usize,
                },
            )),
        ),
        Command::InletCancelled { port_id } => {
            (None, Some((port_id as usize, ConsumerMessage::Cancel)))
        }

        Command::PortPush {
            port_id,
            inner: PortPush { ref items },
        } => (
            Some((
                port_id as usize,
                ProducerMessage::Push {
                    items: items.clone(),
                },
            )),
            None,
        ),

        Command::OutletCompleted { port_id } => {
            (Some((port_id as usize, ProducerMessage::Complete)), None)
        }

        Command::OutletFailed { port_id, ref inner } => (
            Some((
                port_id as usize,
                ProducerMessage::Fail {
                    failure: inner.clone(),
                },
            )),
            None,
        ),

        Command::PortDeclare { .. } => (None, None),
        Command::Hello { .. } => (None, None),
    }
}
