use boxfnonce::SendBoxFnOnce;
use futures::prelude::*;
use futures::sink;
use futures::sync::mpsc::SendError;
use std::collections::VecDeque;

use crate::futures::fsm::*;
use crate::futures::SendBoxedStream;
use crate::protocol::streams::{ProducerRx, ProducerTx};
use crate::protocol::{DataItem, Schema};

use super::*;
use messages::{ConsumerMessage, ProducerMessage};

pub type Src = SendBoxedStream<DataItem, failure::Error>;

#[derive(Debug, Fail)]
pub enum OutletWrapperError {
    #[fail(display = "OutletWrapperError::RxFailure")]
    RxFailure,

    #[fail(display = "OutletWrapperError::TxFailure")]
    TxFailure(SendError<ProducerMessage>),

    #[fail(display = "OutletWrapperError::SerializeFailure")]
    SerializeFailure(#[cause] failure::Error),

    #[fail(display = "OutletWrapperError::UnexpectedConsumerMessage: {:?}", _0)]
    UnexpectedConsumerMessage(ConsumerMessage),

    #[fail(display = "OutletWrapperError::SourceFailure")]
    SourceFailure(#[cause] failure::Error),
    // #[fail(display = "OutletWrapperError::Generic")]
    // Generic(#[cause] failure::Error),
}

type AndThen<Args> = SendBoxFnOnce<'static, Args, TurnOk<OutletWrapper>>;

pub enum OutletWrapper {
    Idle {
        source: Src,
        schema: Schema,
        rx: ProducerRx,
        tx: ProducerTx,
    },
    ProducingItems {
        max_items: usize,
        acc: VecDeque<DataItem>,

        source: Src,
        schema: Schema,
        rx: ProducerRx,
        tx: ProducerTx,
    },

    Sending {
        messages: VecDeque<ProducerMessage>,
        tx: ProducerTx,
        schema: Schema,

        and_then: AndThen<(ProducerTx, Schema)>,
    },
    SendingPollComplete {
        messages: VecDeque<ProducerMessage>,
        sent: sink::Send<ProducerTx>,
        schema: Schema,

        and_then: AndThen<(ProducerTx, Schema)>,
    },

    CheckIfCancelled {
        rx: ProducerRx,
        tx: ProducerTx,
        schema: Schema,
        acc: VecDeque<DataItem>,
        and_then: AndThen<(ProducerRx, ProducerTx, Schema, VecDeque<DataItem>)>,
    },
}

impl OutletWrapper {
    pub fn new(source: Src, schema: Schema) -> (os_process_channels::ConsumerSide, Self) {
        let (producer_side, consumer_side) = os_process_channels::pipe();
        let (producer_rx, producer_tx) = producer_side;

        (
            consumer_side,
            OutletWrapper::Idle {
                rx: producer_rx,
                tx: producer_tx,
                source,
                schema,
            },
        )
    }
}

impl FSM for OutletWrapper {
    type Item = ();
    type Error = OutletWrapperError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            OutletWrapper::Idle {
                mut rx,
                tx,
                schema,
                source,
            } => rx
                .poll()
                .map_err(|()| OutletWrapperError::RxFailure)
                .map(|poll| match poll {
                    Async::NotReady => TurnOk::Suspend(OutletWrapper::Idle {
                        rx,
                        tx,
                        schema,
                        source,
                    }),
                    Async::Ready(None) => {
                        warn!("Rx terminated. Shutting this outlet down");
                        TurnOk::Ready(())
                    }
                    Async::Ready(Some(ConsumerMessage::Cancel)) => {
                        trace!("Received ConsumerMessage::Cancel. Shutting this outlet down");
                        TurnOk::Ready(())
                    }
                    Async::Ready(Some(ConsumerMessage::Pull { max_items })) => {
                        TurnOk::PollMore(OutletWrapper::ProducingItems {
                            max_items,
                            acc: VecDeque::new(),
                            rx,
                            tx,
                            schema,
                            source,
                        })
                    }
                }),

            OutletWrapper::ProducingItems {
                max_items,
                mut acc,
                rx,
                tx,
                schema,
                mut source,
            } => match source.poll() {
                Err(source_err) => {
                    into_failing(tx, OutletWrapperError::SourceFailure(source_err), schema)
                }

                Ok(Async::NotReady) => {
                    if acc.len() == 0 {
                        let max_items = max_items.clone();
                        let into_poll_source = move |rx, tx, schema, acc| {
                            TurnOk::Suspend(OutletWrapper::ProducingItems {
                                max_items,
                                acc,
                                rx,
                                tx,
                                schema,
                                source,
                            })
                        };
                        let check_if_cancelled = OutletWrapper::CheckIfCancelled {
                            rx,
                            tx,
                            acc,
                            schema,
                            and_then: SendBoxFnOnce::from(into_poll_source),
                        };
                        Ok(TurnOk::Suspend(check_if_cancelled))
                    } else {
                        let into_idle = |tx, schema| {
                            TurnOk::PollMore(OutletWrapper::Idle {
                                tx,
                                rx,
                                schema,
                                source,
                            })
                        };
                        into_sending(tx, acc, schema, SendBoxFnOnce::from(into_idle))
                    }
                }

                Ok(Async::Ready(None)) => into_shutting_down(tx, acc, schema),

                Ok(Async::Ready(Some(data_item))) => {
                    acc.push_back(data_item);
                    assert!(acc.len() <= max_items);

                    if acc.len() == max_items {
                        let into_idle = move |tx, schema| {
                            TurnOk::PollMore(OutletWrapper::Idle {
                                tx,
                                schema,
                                rx,
                                source,
                            })
                        };
                        into_sending(tx, acc, schema, SendBoxFnOnce::from(into_idle))
                    } else {
                        Ok(TurnOk::PollMore(OutletWrapper::ProducingItems {
                            max_items,
                            acc,
                            rx,
                            tx,
                            schema,
                            source,
                        }))
                    }
                }
            },

            OutletWrapper::CheckIfCancelled {
                mut rx,
                tx,
                schema,
                acc,
                and_then,
            } => rx
                .poll()
                .map_err(|()| OutletWrapperError::RxFailure)
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(and_then.call(rx, tx, schema, acc)),
                    Async::Ready(None) => into_shutting_down(tx, acc, schema),
                    Async::Ready(Some(ConsumerMessage::Cancel)) => {
                        into_shutting_down(tx, vec![], schema)
                    }

                    Async::Ready(Some(unexpected)) => {
                        Err(OutletWrapperError::UnexpectedConsumerMessage(unexpected))
                    }
                }),

            OutletWrapper::Sending {
                tx,
                schema,
                mut messages,

                and_then,
            } => match messages.pop_front() {
                None => Ok(and_then.call(tx, schema)),

                Some(message) => Ok(TurnOk::PollMore(OutletWrapper::SendingPollComplete {
                    sent: tx.send(message),
                    messages,
                    schema,
                    and_then,
                })),
            },

            OutletWrapper::SendingPollComplete {
                mut sent,
                schema,
                messages,
                and_then,
            } => sent
                .poll()
                .map_err(|tx_err| OutletWrapperError::TxFailure(tx_err))
                .map(|poll| match poll {
                    Async::NotReady => TurnOk::Suspend(OutletWrapper::SendingPollComplete {
                        sent,
                        schema,
                        messages,
                        and_then,
                    }),
                    Async::Ready(tx) => TurnOk::PollMore(OutletWrapper::Sending {
                        tx,
                        schema,
                        messages,
                        and_then,
                    }),
                }),
        }
    }
}

fn into_failing(
    tx: ProducerTx,
    failure: OutletWrapperError,
    schema: Schema,
) -> TurnResult<OutletWrapper> {
    let shutdown = |_, _| TurnOk::Ready(());
    Ok(TurnOk::PollMore(OutletWrapper::Sending {
        tx,
        schema,
        messages: vec![ProducerMessage::Fail {
            failure: Into::<failure::Error>::into(failure).into(),
        }]
        .into(),
        and_then: SendBoxFnOnce::from(shutdown),
    }))
}

fn into_shutting_down<I>(tx: ProducerTx, data_items: I, schema: Schema) -> TurnResult<OutletWrapper>
where
    I: IntoIterator<Item = DataItem>,
{
    let shutdown = |_, _| TurnOk::Ready(());
    let into_completing = move |tx, schema| {
        TurnOk::PollMore(OutletWrapper::Sending {
            tx,
            schema,
            messages: vec![ProducerMessage::Complete].into(),
            and_then: SendBoxFnOnce::from(shutdown),
        })
    };
    into_sending(tx, data_items, schema, SendBoxFnOnce::from(into_completing))
}

fn into_sending<I>(
    tx: ProducerTx,
    data_items: I,
    schema: Schema,
    and_then: AndThen<(ProducerTx, Schema)>,
) -> TurnResult<OutletWrapper>
where
    I: IntoIterator<Item = DataItem>,
{
    data_items
        .into_iter()
        .map(|data_item| avro_rs::to_avro_datum(&schema, data_item))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| OutletWrapperError::SerializeFailure(err.into()))
        .map(|items| ProducerMessage::Push { items })
        .map(|producer_message| {
            let sending = OutletWrapper::Sending {
                tx,
                messages: vec![producer_message].into(),
                schema,
                and_then,
            };
            TurnOk::PollMore(sending)
        })
}
