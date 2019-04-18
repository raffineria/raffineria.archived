use boxfnonce::SendBoxFnOnce;
use futures::future;
use futures::prelude::*;
use futures::sync::mpsc;

use crate::futures::fsm::*;
use crate::futures::{SendBoxedFuture, SendBoxedStream};
use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};

type Continue<Args> = SendBoxFnOnce<'static, Args, TurnResult<TeeFSM>>;

pub enum Event {
    ConsumerMessage(usize, ConsumerMessage),
    ProducerMessage(ProducerMessage),
}

#[derive(Fail, Debug)]
pub enum TeeError {
    #[fail(display = "TeeError::RxError")]
    RxError,

    #[fail(display = "TeeError::InletTxError")]
    InletTxError(#[cause] mpsc::SendError<ConsumerMessage>),

    #[fail(display = "TeeError::OutletTxError")]
    OutletTxError(#[cause] mpsc::SendError<ProducerMessage>),
}

pub type RxEventStream = SendBoxedStream<Event, TeeError>;

#[derive(Debug, Clone)]
pub enum DownstreamState {
    Busy,
    Ready(usize),
}

pub type DownstreamsSend = SendBoxedFuture<Vec<ProducerTx>, mpsc::SendError<ProducerMessage>>;
pub type UpstreamSend = SendBoxedFuture<ConsumerTx, mpsc::SendError<ConsumerMessage>>;

pub enum TeeFSM {
    SendingToDownstreams {
        sents: DownstreamsSend,
        and_then: Continue<(Vec<ProducerTx>,)>,
    },
    SendingToUpstream {
        sent: UpstreamSend,
        and_then: Continue<(ConsumerTx,)>,
    },
    ReceiveEvent {
        rx_events: RxEventStream,
        inlet_tx: ConsumerTx,
        outlet_txs: Vec<ProducerTx>,
        downstream_states: Vec<DownstreamState>,
    },
}

impl FSM for TeeFSM {
    type Item = ();
    type Error = TeeError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            TeeFSM::ReceiveEvent {
                mut rx_events,
                inlet_tx,
                outlet_txs,
                downstream_states,
            } => rx_events.poll().and_then(|poll| match poll {
                Async::NotReady => Ok(TurnOk::Suspend(TeeFSM::ReceiveEvent {
                    rx_events,
                    inlet_tx,
                    outlet_txs,
                    downstream_states,
                })),

                Async::Ready(None) => Ok(TurnOk::Ready(())),

                Async::Ready(Some(event)) => {
                    handle_rx_event(event, rx_events, inlet_tx, outlet_txs, downstream_states)
                }
            }),

            TeeFSM::SendingToDownstreams {
                mut sents,
                and_then,
            } => sents
                .poll()
                .map_err(|err| TeeError::OutletTxError(err))
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(TeeFSM::SendingToDownstreams {
                        sents,
                        and_then,
                    })),
                    Async::Ready(outlet_txs) => and_then.call(outlet_txs),
                }),

            TeeFSM::SendingToUpstream { mut sent, and_then } => sent
                .poll()
                .map_err(|err| TeeError::InletTxError(err))
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(TeeFSM::SendingToUpstream {
                        sent,
                        and_then,
                    })),
                    Async::Ready(inlet_tx) => and_then.call(inlet_tx),
                }),
        }
    }
}

impl TeeFSM {
    pub fn new(inlet: (ConsumerTx, ConsumerRx), outlets: Vec<(ProducerTx, ProducerRx)>) -> Self {
        let (inlet_tx, inlet_rx) = inlet;
        let (outlet_txs, outlet_rxs): (Vec<_>, Vec<_>) = outlets.into_iter().unzip();
        let downstream_states = outlet_txs.iter().map(|_| DownstreamState::Busy).collect();

        let rx_events = rxs_into_event_stream(inlet_rx, outlet_rxs);

        TeeFSM::ReceiveEvent {
            rx_events,
            inlet_tx,
            outlet_txs,
            downstream_states,
        }
    }
}

fn handle_rx_event(
    event: Event,
    rx_events: RxEventStream,
    inlet_tx: ConsumerTx,
    outlet_txs: Vec<ProducerTx>,
    downstream_states: Vec<DownstreamState>,
) -> TurnResult<TeeFSM> {
    match event {
        Event::ProducerMessage(producer_message) => handle_rx_event_producer_message(
            producer_message,
            rx_events,
            inlet_tx,
            outlet_txs,
            downstream_states,
        ),

        Event::ConsumerMessage(consumer_idx, consumer_message) => handle_rx_event_consumer_message(
            consumer_idx,
            consumer_message,
            rx_events,
            inlet_tx,
            outlet_txs,
            downstream_states,
        ),
    }
}

fn handle_rx_event_producer_message(
    producer_message: ProducerMessage,
    rx_events: RxEventStream,
    inlet_tx: ConsumerTx,
    outlet_txs: Vec<ProducerTx>,
    _downstream_states: Vec<DownstreamState>,
) -> TurnResult<TeeFSM> {
    match producer_message {
        ProducerMessage::Fail { failure } => {
            let shutdown = |_outlet_txs| Ok(TurnOk::Ready(()));
            let sents = Box::new(future::join_all(outlet_txs.into_iter().map(
                move |outlet_tx| {
                    outlet_tx.send(ProducerMessage::Fail {
                        failure: failure.clone(),
                    })
                },
            )));
            Ok(TurnOk::PollMore(TeeFSM::SendingToDownstreams {
                sents,
                and_then: SendBoxFnOnce::from(shutdown),
            }))
        }

        ProducerMessage::Complete => {
            let shutdown = |_outlet_txs| Ok(TurnOk::Ready(()));
            let sents = Box::new(future::join_all(
                outlet_txs
                    .into_iter()
                    .map(|outlet_tx| outlet_tx.send(ProducerMessage::Complete)),
            ));
            Ok(TurnOk::PollMore(TeeFSM::SendingToDownstreams {
                sents,
                and_then: SendBoxFnOnce::from(shutdown),
            }))
        }

        ProducerMessage::Push { items } => {
            let downstream_states = outlet_txs.iter().map(|_| DownstreamState::Busy).collect();
            let into_receiving_events = move |outlet_txs| {
                Ok(TurnOk::PollMore(TeeFSM::ReceiveEvent {
                    rx_events,
                    inlet_tx,
                    outlet_txs,
                    downstream_states,
                }))
            };
            let sents = Box::new(future::join_all(outlet_txs.into_iter().map(
                move |outlet_tx| {
                    outlet_tx.send(ProducerMessage::Push {
                        items: items.clone(),
                    })
                },
            )));
            Ok(TurnOk::PollMore(TeeFSM::SendingToDownstreams {
                sents,
                and_then: SendBoxFnOnce::from(into_receiving_events),
            }))
        }
    }
}

fn handle_rx_event_consumer_message(
    consumer_idx: usize,
    consumer_message: ConsumerMessage,
    rx_events: RxEventStream,
    inlet_tx: ConsumerTx,
    outlet_txs: Vec<ProducerTx>,
    mut downstream_states: Vec<DownstreamState>,
) -> TurnResult<TeeFSM> {
    assert!(consumer_idx < downstream_states.len());
    match consumer_message {
        ConsumerMessage::Cancel => {
            let cancel_upstream_sent = Box::new(inlet_tx.send(ConsumerMessage::Cancel));
            let into_completing_downstreams = move |_inlet_tx| {
                let shutdown = |_outlet_txs| Ok(TurnOk::Ready(()));
                let complete_downstreams_sent = Box::new(future::join_all(
                    outlet_txs
                        .into_iter()
                        .map(|outlet_tx| outlet_tx.send(ProducerMessage::Complete)),
                ));

                Ok(TurnOk::PollMore(TeeFSM::SendingToDownstreams {
                    sents: complete_downstreams_sent,
                    and_then: SendBoxFnOnce::from(shutdown),
                }))
            };
            Ok(TurnOk::PollMore(TeeFSM::SendingToUpstream {
                sent: cancel_upstream_sent,
                and_then: SendBoxFnOnce::from(into_completing_downstreams),
            }))
        }

        ConsumerMessage::Pull { max_items } => {
            downstream_states[consumer_idx] = DownstreamState::Ready(max_items);
            match should_pull_upstream(&downstream_states) {
                Some(non_zero) if non_zero > 0 => {
                    let sent = Box::new(inlet_tx.send(ConsumerMessage::Pull {
                        max_items: non_zero,
                    }));
                    let into_receiving_events = move |inlet_tx| {
                        Ok(TurnOk::PollMore(TeeFSM::ReceiveEvent {
                            inlet_tx,
                            outlet_txs,
                            rx_events,
                            downstream_states,
                        }))
                    };
                    Ok(TurnOk::PollMore(TeeFSM::SendingToUpstream {
                        sent,
                        and_then: SendBoxFnOnce::from(into_receiving_events),
                    }))
                }
                _ => Ok(TurnOk::PollMore(TeeFSM::ReceiveEvent {
                    rx_events,
                    inlet_tx,
                    outlet_txs,
                    downstream_states,
                })),
            }
        }
    }
}

fn should_pull_upstream(downstream_states: &Vec<DownstreamState>) -> Option<usize> {
    downstream_states
        .iter()
        .fold(None, |acc, state| match (acc, state) {
            (_, &DownstreamState::Busy) => Some(0),
            (None, &DownstreamState::Ready(ref max_items)) => Some(*max_items),
            (Some(acc), &DownstreamState::Ready(ref max_items)) => {
                Some(std::cmp::min(acc, *max_items))
            }
        })
}

fn rxs_into_event_stream(
    inlet_rx: ConsumerRx,
    outlet_rxs: Vec<ProducerRx>,
) -> SendBoxedStream<Event, TeeError> {
    let inlet_rx_events = inlet_rx
        .map(|producer_message| Event::ProducerMessage(producer_message))
        .map_err(|()| TeeError::RxError);
    let outlet_rx_events =
        outlet_rxs
            .into_iter()
            .enumerate()
            .map(move |(outlet_idx, outlet_rx)| {
                outlet_rx
                    .map(move |consumer_message| {
                        Event::ConsumerMessage(outlet_idx, consumer_message)
                    })
                    .map_err(|()| TeeError::RxError)
            });

    outlet_rx_events
        .fold::<SendBoxedStream<Event, TeeError>, _>(Box::new(inlet_rx_events), |acc, outlet_rx| {
            Box::new(acc.select(outlet_rx))
        })
}
