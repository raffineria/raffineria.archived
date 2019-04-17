use futures::prelude::*;
use futures::stream;

use crate::futures::SendBoxedStream;
use crate::protocol::command::{PortPull, PortPush};
use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::Command;

use super::*;
use message_channels::{ConsumerRx, ProducerRx};

type CommandsStream = SendBoxedStream<Command, MessageToCommandError>;

#[derive(Fail, Debug)]
pub enum MessageToCommandError {
    #[fail(
        display = "MessageToCommandError::ProducerRxFailure [inlet_idx: {}]",
        inlet_idx
    )]
    ProducerRxFailure { inlet_idx: usize },

    #[fail(
        display = "MessageToCommandError::ConsumerRxFailure [outlet_idx: {}]",
        outlet_idx
    )]
    ConsumerRxFailure { outlet_idx: usize },
}

pub struct MessageToCommand {
    commands_stream: CommandsStream,
}

impl MessageToCommand {
    pub fn new(producer_rxs: Vec<ProducerRx>, consumer_rxs: Vec<ConsumerRx>) -> Self {
        let producer_command_streams_merged = producer_rxs
            .into_iter()
            .enumerate()
            .map(|(inlet_idx, producer_rx)| {
                producer_rx
                    .map_err(move |()| MessageToCommandError::ProducerRxFailure { inlet_idx })
                    .map(move |message| match message {
                        ConsumerMessage::Pull { max_items } => Command::PortPull {
                            port_id: inlet_idx as i32,
                            inner: PortPull {
                                max_items: max_items as i32,
                            },
                        },
                        ConsumerMessage::Cancel => Command::InletCancelled {
                            port_id: inlet_idx as i32,
                        },
                    })
            })
            .fold::<CommandsStream, _>(Box::new(stream::empty()), |left, right| {
                Box::new(left.select(right))
            });

        let consumer_command_streams_merged = consumer_rxs
            .into_iter()
            .enumerate()
            .map(|(outlet_idx, consumer_rx)| {
                consumer_rx
                    .map_err(move |()| MessageToCommandError::ConsumerRxFailure { outlet_idx })
                    .map(move |message| match message {
                        ProducerMessage::Push { items } => Command::PortPush {
                            port_id: outlet_idx as i32,
                            inner: PortPush { items },
                        },
                        ProducerMessage::Complete => Command::OutletCompleted {
                            port_id: outlet_idx as i32,
                        },
                        ProducerMessage::Fail { failure } => Command::OutletFailed {
                            port_id: outlet_idx as i32,
                            inner: failure.into(),
                        },
                    })
            })
            .fold::<CommandsStream, _>(Box::new(stream::empty()), |left, right| {
                Box::new(left.select(right))
            });

        let commands_stream =
            Box::new(producer_command_streams_merged.select(consumer_command_streams_merged));

        Self { commands_stream }
    }
}

impl Stream for MessageToCommand {
    type Item = <CommandsStream as Stream>::Item;
    type Error = <CommandsStream as Stream>::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.commands_stream.poll().map(|poll| {
            poll.map(|item| {
                trace!("<MessageToCommand as Stream>::poll(...) -> {:?}", item);
                item
            })
        })
    }
}
