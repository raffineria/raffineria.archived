use boxfnonce::SendBoxFnOnce;
use futures::prelude::*;
use futures::sink;
use futures::sync::mpsc::SendError;
use std::collections::VecDeque;

use crate::futures::fsm::*;
use crate::futures::SendBoxedSink;
use crate::protocol::command::Failure as PortFailure;
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::{DataItem, Schema};

use super::*;
use messages::{ConsumerMessage, ProducerMessage};

pub type Snk = SendBoxedSink<DataItem, failure::Error>;

const PULL_MAX_ITEMS: usize = 16;

#[derive(Fail, Debug)]
pub enum InletWrapperError {
    #[fail(display = "InletWrapperError::SinkFailure")]
    SinkFailure(#[cause] failure::Error),

    #[fail(display = "InletWrapperError::TxFailure: {:?}", _0)]
    TxFailure(SendError<ConsumerMessage>),

    #[fail(display = "InletWrapperError::RxFailure")]
    RxFailure,

    #[fail(display = "InletWrapperError::InletFailure: {:?}", _0)]
    InletFailure(PortFailure),

    #[fail(display = "InletWrapperError::DeserializeFailure")]
    DeserializeFailure(#[cause] failure::Error),
}

type AndThen<Args> = SendBoxFnOnce<'static, Args, TurnResult<InletWrapper>>;

pub enum InletWrapper {
    SinkPollComplete {
        sent: sink::Send<Snk>,

        schema: Schema,
        rx: ConsumerRx,
        tx: ConsumerTx,
        items: VecDeque<DataItem>,
    },
    SinkSend {
        sink: Snk,
        schema: Schema,
        rx: ConsumerRx,
        tx: ConsumerTx,

        items: VecDeque<DataItem>,
    },

    Pulling {
        sink: Snk,
        schema: Schema,
        rx: ConsumerRx,
        tx: ConsumerTx,
    },

    TxSend {
        messages: VecDeque<ConsumerMessage>,
        tx: ConsumerTx,
        and_then: AndThen<(ConsumerTx,)>,
    },

    TxPollComplete {
        sent: sink::Send<ConsumerTx>,
        messages: VecDeque<ConsumerMessage>,
        and_then: AndThen<(ConsumerTx,)>,
    },

    WaitingForPush {
        sink: Snk,
        schema: Schema,
        rx: ConsumerRx,
        tx: ConsumerTx,
    },
}

impl InletWrapper {
    pub fn new(sink: Snk, schema: Schema) -> (os_process_channels::ProducerSide, Self) {
        let (producer_side, consumer_side) = os_process_channels::pipe();
        let (consumer_rx, consumer_tx) = consumer_side;

        (
            producer_side,
            InletWrapper::SinkSend {
                rx: consumer_rx,
                tx: consumer_tx,
                sink,
                schema,
                items: vec![].into(),
            },
        )
    }
}

impl FSM for InletWrapper {
    type Item = ();
    type Error = InletWrapperError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            InletWrapper::SinkPollComplete {
                mut sent,
                schema,
                rx,
                tx,

                items,
            } => match sent
                .poll()
                .map_err(|sink_failure| InletWrapperError::SinkFailure(sink_failure))
            {
                Err(reason) => {
                    let message = ConsumerMessage::Cancel;
                    let shutdown = move |_| Err(reason);

                    let sending_cancel = InletWrapper::TxSend {
                        messages: vec![message].into(),
                        tx,
                        and_then: SendBoxFnOnce::from(shutdown),
                    };
                    Ok(TurnOk::PollMore(sending_cancel))
                }

                Ok(Async::NotReady) => Ok(TurnOk::Suspend(InletWrapper::SinkPollComplete {
                    sent,
                    schema,
                    rx,
                    tx,
                    items,
                })),

                Ok(Async::Ready(sink)) => Ok(TurnOk::PollMore(InletWrapper::SinkSend {
                    sink,
                    schema,
                    rx,
                    tx,
                    items,
                })),
            },

            InletWrapper::SinkSend {
                sink,
                schema,
                rx,
                tx,
                mut items,
            } => Ok(match items.pop_front() {
                None => TurnOk::PollMore(InletWrapper::Pulling {
                    sink,
                    schema,
                    rx,
                    tx,
                }),
                Some(item) => {
                    let sent = sink.send(item);
                    TurnOk::PollMore(InletWrapper::SinkPollComplete {
                        sent,
                        schema,
                        rx,
                        tx,
                        items,
                    })
                }
            }),

            InletWrapper::Pulling {
                sink,
                schema,
                rx,
                tx,
            } => {
                let message = ConsumerMessage::Pull {
                    max_items: PULL_MAX_ITEMS,
                };
                let into_waiting_for_push = move |tx| {
                    Ok(TurnOk::PollMore(InletWrapper::WaitingForPush {
                        tx,
                        rx,
                        schema,
                        sink,
                    }))
                };

                Ok(TurnOk::PollMore(InletWrapper::TxSend {
                    tx,
                    messages: vec![message].into(),
                    and_then: SendBoxFnOnce::from(into_waiting_for_push),
                }))
            }

            InletWrapper::TxSend {
                tx,
                mut messages,
                and_then,
            } => match messages.pop_front() {
                None => and_then.call(tx),
                Some(message) => {
                    let sent = tx.send(message);
                    Ok(TurnOk::PollMore(InletWrapper::TxPollComplete {
                        sent,
                        messages,
                        and_then,
                    }))
                }
            },

            InletWrapper::TxPollComplete {
                mut sent,
                messages,
                and_then,
            } => sent
                .poll()
                .map_err(|tx_failure| InletWrapperError::TxFailure(tx_failure))
                .map(|poll| match poll {
                    Async::NotReady => TurnOk::Suspend(InletWrapper::TxPollComplete {
                        sent,
                        messages,
                        and_then,
                    }),
                    Async::Ready(tx) => TurnOk::PollMore(InletWrapper::TxSend {
                        tx,
                        messages,
                        and_then,
                    }),
                }),

            InletWrapper::WaitingForPush {
                sink,
                schema,
                mut rx,
                tx,
            } => rx
                .poll()
                .map_err(|()| InletWrapperError::RxFailure)
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(InletWrapper::WaitingForPush {
                        sink,
                        schema,
                        rx,
                        tx,
                    })),
                    Async::Ready(None) => Err(InletWrapperError::RxFailure),
                    Async::Ready(Some(ProducerMessage::Complete)) => Ok(TurnOk::Ready(())),
                    Async::Ready(Some(ProducerMessage::Fail { failure })) => {
                        Err(InletWrapperError::InletFailure(failure))
                    }
                    Async::Ready(Some(ProducerMessage::Push { items })) => items
                        .into_iter()
                        .map(|item| {
                            use bytes::IntoBuf;
                            avro_rs::from_avro_datum(&schema, &mut item.into_buf(), None)
                        })
                        .collect::<Result<VecDeque<_>, _>>()
                        .map(|items| {
                            TurnOk::PollMore(InletWrapper::SinkSend {
                                rx,
                                tx,
                                schema,
                                sink,
                                items,
                            })
                        })
                        .map_err(|de_err| InletWrapperError::DeserializeFailure(de_err.into())),
                }),
        }
    }
}
