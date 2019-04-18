use std::collections::VecDeque;

use futures::future;
use futures::prelude::*;
use futures::sync::mpsc;

use boxfnonce::SendBoxFnOnce;

use crate::futures::fsm::*;
use crate::futures::{SendBoxedFuture, SendBoxedStream};
use crate::protocol::command::Failure as PortFailure;
use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};

type Continue<Args> = SendBoxFnOnce<'static, Args, TurnResult<MergeFSM>>;

pub enum Event {
    ConsumerMessage(ConsumerMessage),
    ProducerMessage(usize, ProducerMessage),
}

#[derive(Fail, Debug)]
pub enum MergeError {
    #[fail(display = "MergeError::RxError")]
    RxError,

    #[fail(display = "MergeError::InletTxError")]
    InletTxError(#[cause] mpsc::SendError<ConsumerMessage>),

    #[fail(display = "MergeError::OutletTxError")]
    OutletTxError(#[cause] mpsc::SendError<ProducerMessage>),

    #[fail(display = "MergeError::PulledTwice")]
    PulledTwice,
}

pub type RxEventStream = SendBoxedStream<Event, MergeError>;

pub type DownstreamSend = SendBoxedFuture<ProducerTx, mpsc::SendError<ProducerMessage>>;
pub type UpstreamSendSingle = SendBoxedFuture<ConsumerTx, mpsc::SendError<ConsumerMessage>>;
pub type UpstreamsSend = SendBoxedFuture<Vec<ConsumerTx>, mpsc::SendError<ConsumerMessage>>;

pub type ItemsBuffer = VecDeque<Vec<u8>>;

#[derive(Debug, Clone)]
pub enum UpstreamState {
    Pulled,
    Idle,
    Complete,
    Failed(PortFailure),
}

pub enum MergeFSM {
    Busy {
        eagerly_complete: bool,
        eagerly_fail: bool,
        buffer: ItemsBuffer,
        rx_events: RxEventStream,
        outlet_tx: ProducerTx,
        inlet_txs: Vec<ConsumerTx>,
        upstream_states: Vec<UpstreamState>,
    },
    WaitingForUpstreams {
        eagerly_complete: bool,
        eagerly_fail: bool,
        max_items: usize,
        rx_events: RxEventStream,
        outlet_tx: ProducerTx,
        inlet_txs: Vec<ConsumerTx>,
        upstream_states: Vec<UpstreamState>,
    },
    SendingToUpstreams {
        sents: UpstreamsSend,
        and_then: Continue<(Vec<ConsumerTx>,)>,
    },
    SendingToDownstream {
        sent: DownstreamSend,
        and_then: Continue<(ProducerTx,)>,
    },
}

impl MergeFSM {
    pub fn new(
        inlets: Vec<(ConsumerTx, ConsumerRx)>,
        outlet: (ProducerTx, ProducerRx),
        eagerly_complete: bool,
        eagerly_fail: bool,
    ) -> Self {
        let (outlet_tx, outlet_rx) = outlet;
        let (inlet_txs, inlet_rxs): (Vec<_>, Vec<_>) = inlets.into_iter().unzip();

        let rx_events = rxs_into_event_stream(inlet_rxs, outlet_rx);

        let upstream_states = inlet_txs.iter().map(|_| UpstreamState::Idle).collect();

        MergeFSM::Busy {
            eagerly_complete,
            eagerly_fail,
            buffer: VecDeque::new(),
            rx_events,
            outlet_tx,
            inlet_txs,
            upstream_states,
        }
    }
}

impl FSM for MergeFSM {
    type Item = ();
    type Error = MergeError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            MergeFSM::SendingToUpstreams {
                mut sents,
                and_then,
            } => sents
                .poll()
                .map_err(|err| MergeError::InletTxError(err))
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(MergeFSM::SendingToUpstreams {
                        sents,
                        and_then,
                    })),

                    Async::Ready(inlet_txs) => and_then.call(inlet_txs),
                }),

            MergeFSM::SendingToDownstream { mut sent, and_then } => sent
                .poll()
                .map_err(|err| MergeError::OutletTxError(err))
                .and_then(|poll| match poll {
                    Async::NotReady => Ok(TurnOk::Suspend(MergeFSM::SendingToDownstream {
                        sent,
                        and_then,
                    })),

                    Async::Ready(outlet_tx) => and_then.call(outlet_tx),
                }),

            MergeFSM::WaitingForUpstreams {
                eagerly_complete,
                eagerly_fail,
                max_items,
                outlet_tx,
                inlet_txs,
                mut rx_events,
                upstream_states,
            } => rx_events.poll().and_then(|poll| match poll {
                Async::NotReady => Ok(TurnOk::Suspend(MergeFSM::WaitingForUpstreams {
                    eagerly_complete,
                    eagerly_fail,
                    max_items,
                    outlet_tx,
                    inlet_txs,
                    rx_events,
                    upstream_states,
                })),

                Async::Ready(None) => Ok(TurnOk::Ready(())),

                Async::Ready(Some(event)) => handle_rx_event_while_waiting_for_upstreams(
                    event,
                    eagerly_complete,
                    eagerly_fail,
                    max_items,
                    outlet_tx,
                    inlet_txs,
                    rx_events,
                    upstream_states,
                ),
            }),

            MergeFSM::Busy {
                eagerly_complete,
                eagerly_fail,
                buffer,
                outlet_tx,
                inlet_txs,
                mut rx_events,
                upstream_states,
            } => rx_events.poll().and_then(|poll| match poll {
                Async::NotReady => Ok(TurnOk::Suspend(MergeFSM::Busy {
                    eagerly_complete,
                    eagerly_fail,
                    buffer,
                    outlet_tx,
                    inlet_txs,
                    rx_events,
                    upstream_states,
                })),

                Async::Ready(None) => Ok(TurnOk::Ready(())),

                Async::Ready(Some(event)) => handle_rx_event_while_busy(
                    event,
                    eagerly_complete,
                    eagerly_fail,
                    outlet_tx,
                    inlet_txs,
                    rx_events,
                    buffer,
                    upstream_states,
                ),
            }),
        }
    }
}

fn handle_rx_event_while_waiting_for_upstreams(
    event: Event,

    eagerly_complete: bool,
    eagerly_fail: bool,
    max_items: usize,
    outlet_tx: ProducerTx,
    inlet_txs: Vec<ConsumerTx>,
    rx_events: RxEventStream,
    mut upstream_states: Vec<UpstreamState>,
) -> TurnResult<MergeFSM> {
    match event {
        Event::ConsumerMessage(ConsumerMessage::Pull { .. }) => Err(MergeError::PulledTwice),

        Event::ConsumerMessage(ConsumerMessage::Cancel) => {
            let send_downstream_complete = move |_inelt_txs| {
                let shutdown = move |_outlet_tx| Ok(TurnOk::Ready(()));
                let downstream_complete_sent = Box::new(outlet_tx.send(ProducerMessage::Complete));
                Ok(TurnOk::PollMore(MergeFSM::SendingToDownstream {
                    sent: downstream_complete_sent,
                    and_then: SendBoxFnOnce::from(shutdown),
                }))
            };
            let upstream_cancel_sents = Box::new(future::join_all(
                inlet_txs
                    .into_iter()
                    .map(|inlet_tx| inlet_tx.send(ConsumerMessage::Cancel)),
            ));
            Ok(TurnOk::PollMore(MergeFSM::SendingToUpstreams {
                sents: upstream_cancel_sents,
                and_then: SendBoxFnOnce::from(send_downstream_complete),
            }))
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Complete) => {
            upstream_states[producer_idx] = UpstreamState::Complete;
            if eagerly_complete {
                shutdown_eagerly(
                    upstream_states,
                    ProducerMessage::Complete,
                    outlet_tx,
                    inlet_txs,
                )
            } else {
                shutdown_if_no_upstreams_left(
                    upstream_states,
                    outlet_tx,
                    inlet_txs,
                    SendBoxFnOnce::from(move |upstream_states, outlet_tx, inlet_txs| {
                        Ok(TurnOk::PollMore(MergeFSM::WaitingForUpstreams {
                            upstream_states,
                            outlet_tx,
                            inlet_txs,
                            rx_events,
                            eagerly_complete,
                            eagerly_fail,
                            max_items,
                        }))
                    }),
                )
            }
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Fail { failure }) => {
            upstream_states[producer_idx] = UpstreamState::Failed(failure.clone());
            if eagerly_fail {
                shutdown_eagerly(
                    upstream_states,
                    ProducerMessage::Fail { failure },
                    outlet_tx,
                    inlet_txs,
                )
            } else {
                shutdown_if_no_upstreams_left(
                    upstream_states,
                    outlet_tx,
                    inlet_txs,
                    SendBoxFnOnce::from(move |upstream_states, outlet_tx, inlet_txs| {
                        Ok(TurnOk::PollMore(MergeFSM::WaitingForUpstreams {
                            upstream_states,
                            outlet_tx,
                            inlet_txs,
                            rx_events,
                            eagerly_complete,
                            eagerly_fail,
                            max_items,
                        }))
                    }),
                )
            }
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Push { items }) => {
            upstream_states[producer_idx] = UpstreamState::Idle;

            let items_available = items.len();
            let items_to_send = std::cmp::min(max_items, items_available);
            let mut items: VecDeque<_> = items.into();
            let items_to_send = items.drain(0..items_to_send).collect();
            let push_message = ProducerMessage::Push {
                items: items_to_send,
            };

            let downstream_push_sent = Box::new(outlet_tx.send(push_message));

            let into_busy = move |outlet_tx| {
                Ok(TurnOk::PollMore(MergeFSM::Busy {
                    buffer: items,
                    outlet_tx,
                    inlet_txs,
                    rx_events,
                    upstream_states,
                    eagerly_complete,
                    eagerly_fail,
                }))
            };

            Ok(TurnOk::PollMore(MergeFSM::SendingToDownstream {
                sent: downstream_push_sent,
                and_then: SendBoxFnOnce::from(into_busy),
            }))
        }
    }
}

fn handle_rx_event_while_busy(
    event: Event,

    eagerly_complete: bool,
    eagerly_fail: bool,
    outlet_tx: ProducerTx,
    inlet_txs: Vec<ConsumerTx>,
    rx_events: RxEventStream,

    mut buffer: ItemsBuffer,
    mut upstream_states: Vec<UpstreamState>,
) -> TurnResult<MergeFSM> {
    match event {
        Event::ConsumerMessage(ConsumerMessage::Pull { max_items }) => {
            if buffer.is_empty() {
                assert!(inlet_txs.len() == upstream_states.len());

                let (upstream_states, upstream_pull_sents): (Vec<_>, Vec<_>) = upstream_states
                    .into_iter()
                    .zip(inlet_txs.into_iter())
                    .map::<(UpstreamState, UpstreamSendSingle), _>(|(upstream_state, inlet_tx)| {
                        match upstream_state {
                            UpstreamState::Idle => (
                                UpstreamState::Pulled,
                                Box::new(inlet_tx.send(ConsumerMessage::Pull { max_items })),
                            ),

                            as_is => (as_is, Box::new(future::ok(inlet_tx))),
                        }
                    })
                    .unzip();
                let upstream_pull_sents = Box::new(future::join_all(upstream_pull_sents));

                let into_waiting_for_upstreams = move |inlet_txs| {
                    Ok(TurnOk::PollMore(MergeFSM::WaitingForUpstreams {
                        inlet_txs,
                        outlet_tx,
                        rx_events,
                        max_items,
                        upstream_states,
                        eagerly_complete,
                        eagerly_fail,
                    }))
                };

                Ok(TurnOk::PollMore(MergeFSM::SendingToUpstreams {
                    sents: upstream_pull_sents,
                    and_then: SendBoxFnOnce::from(into_waiting_for_upstreams),
                }))
            } else {
                let items_available = buffer.len();
                let items_to_send = std::cmp::min(items_available, max_items);
                let items_to_send = buffer.drain(0..items_to_send).collect();
                let push_sent = Box::new(outlet_tx.send(ProducerMessage::Push {
                    items: items_to_send,
                }));

                let into_busy = move |outlet_tx| {
                    Ok(TurnOk::PollMore(MergeFSM::Busy {
                        outlet_tx,
                        inlet_txs,
                        rx_events,

                        buffer,
                        upstream_states,
                        eagerly_complete,
                        eagerly_fail,
                    }))
                };

                Ok(TurnOk::PollMore(MergeFSM::SendingToDownstream {
                    sent: push_sent,
                    and_then: SendBoxFnOnce::from(into_busy),
                }))
            }
        }

        Event::ConsumerMessage(ConsumerMessage::Cancel) => {
            let send_downstream_complete = move |_inlet_txs| {
                let shutdown = |_outlet_tx| Ok(TurnOk::Ready(()));
                let downstream_complete_sent = Box::new(outlet_tx.send(ProducerMessage::Complete));
                Ok(TurnOk::PollMore(MergeFSM::SendingToDownstream {
                    sent: downstream_complete_sent,
                    and_then: SendBoxFnOnce::from(shutdown),
                }))
            };
            let upstream_cancel_sents = Box::new(future::join_all(
                inlet_txs
                    .into_iter()
                    .map(|inlet_tx| inlet_tx.send(ConsumerMessage::Cancel)),
            ));

            Ok(TurnOk::PollMore(MergeFSM::SendingToUpstreams {
                sents: upstream_cancel_sents,
                and_then: SendBoxFnOnce::from(send_downstream_complete),
            }))
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Complete) => {
            upstream_states[producer_idx] = UpstreamState::Complete;
            if eagerly_complete {
                shutdown_eagerly(
                    upstream_states,
                    ProducerMessage::Complete,
                    outlet_tx,
                    inlet_txs,
                )
            } else {
                shutdown_if_no_upstreams_left(
                    upstream_states,
                    outlet_tx,
                    inlet_txs,
                    SendBoxFnOnce::from(move |upstream_states, outlet_tx, inlet_txs| {
                        Ok(TurnOk::PollMore(MergeFSM::Busy {
                            buffer,
                            upstream_states,
                            outlet_tx,
                            inlet_txs,
                            rx_events,
                            eagerly_complete,
                            eagerly_fail,
                        }))
                    }),
                )
            }
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Fail { failure }) => {
            upstream_states[producer_idx] = UpstreamState::Failed(failure.clone());
            if eagerly_fail {
                shutdown_eagerly(
                    upstream_states,
                    ProducerMessage::Fail { failure },
                    outlet_tx,
                    inlet_txs,
                )
            } else {
                shutdown_if_no_upstreams_left(
                    upstream_states,
                    outlet_tx,
                    inlet_txs,
                    SendBoxFnOnce::from(move |upstream_states, outlet_tx, inlet_txs| {
                        Ok(TurnOk::PollMore(MergeFSM::Busy {
                            buffer,
                            upstream_states,
                            outlet_tx,
                            inlet_txs,
                            rx_events,
                            eagerly_complete,
                            eagerly_fail,
                        }))
                    }),
                )
            }
        }

        Event::ProducerMessage(producer_idx, ProducerMessage::Push { items }) => {
            upstream_states[producer_idx] = UpstreamState::Idle;
            buffer.append(&mut items.into());

            Ok(TurnOk::PollMore(MergeFSM::Busy {
                rx_events,
                outlet_tx,
                inlet_txs,
                buffer,
                upstream_states,
                eagerly_complete,
                eagerly_fail,
            }))
        }
    }
}

fn shutdown_if_no_upstreams_left(
    upstream_states: Vec<UpstreamState>,
    outlet_tx: ProducerTx,
    inlet_txs: Vec<ConsumerTx>,
    or_else: Continue<(Vec<UpstreamState>, ProducerTx, Vec<ConsumerTx>)>,
) -> TurnResult<MergeFSM> {
    if upstream_states
        .iter()
        .any(|upstream_state| match upstream_state {
            UpstreamState::Complete => false,
            UpstreamState::Failed { .. } => false,
            UpstreamState::Idle => true,
            UpstreamState::Pulled => true,
        })
    {
        or_else.call(upstream_states, outlet_tx, inlet_txs)
    } else {
        shutdown_eagerly(
            upstream_states,
            ProducerMessage::Complete,
            outlet_tx,
            inlet_txs,
        )
    }
}

fn shutdown_eagerly(
    upstream_states: Vec<UpstreamState>,
    downstream_bye_message: ProducerMessage,
    outlet_tx: ProducerTx,
    inlet_txs: Vec<ConsumerTx>,
) -> TurnResult<MergeFSM> {
    assert!(upstream_states.len() == inlet_txs.len());

    let upstream_cancel_sents = upstream_states
        .into_iter()
        .zip(inlet_txs.into_iter())
        .map::<UpstreamSendSingle, _>(|(upstream_state, inlet_tx)| {
        let should_cancel = match upstream_state {
            UpstreamState::Idle => true,
            UpstreamState::Pulled => true,
            UpstreamState::Failed { .. } => false,
            UpstreamState::Complete => false,
        };
        if should_cancel {
            Box::new(inlet_tx.send(ConsumerMessage::Cancel))
        } else {
            Box::new(future::ok(inlet_tx))
        }
    });
    let upstream_cancel_sents = Box::new(future::join_all(upstream_cancel_sents));

    let send_downstream_termination = move |_inlet_txs| {
        let downstream_terminate_sent = Box::new(outlet_tx.send(downstream_bye_message));

        let shutdown = |_outlet_tx| Ok(TurnOk::Ready(()));

        Ok(TurnOk::PollMore(MergeFSM::SendingToDownstream {
            sent: downstream_terminate_sent,
            and_then: SendBoxFnOnce::from(shutdown),
        }))
    };

    Ok(TurnOk::PollMore(MergeFSM::SendingToUpstreams {
        sents: upstream_cancel_sents,
        and_then: SendBoxFnOnce::from(send_downstream_termination),
    }))
}

fn rxs_into_event_stream(
    inlet_rxs: Vec<ConsumerRx>,
    outlet_rx: ProducerRx,
) -> SendBoxedStream<Event, MergeError> {
    let outlet_rx_events = outlet_rx
        .map(|consumer_message| Event::ConsumerMessage(consumer_message))
        .map_err(|()| MergeError::RxError);

    let inlet_rx_events = inlet_rxs
        .into_iter()
        .enumerate()
        .map(move |(inlet_idx, inlet_rx)| {
            inlet_rx
                .map(move |producer_message| Event::ProducerMessage(inlet_idx, producer_message))
                .map_err(|()| MergeError::RxError)
        });

    inlet_rx_events.fold::<SendBoxedStream<Event, MergeError>, _>(
        Box::new(outlet_rx_events),
        |acc, inlet_rx| Box::new(acc.select(inlet_rx)),
    )
}
