use futures::sync::mpsc;

use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::streams::{ConsumerRx, ConsumerTx};
use crate::protocol::streams::{ProducerRx, ProducerTx};

pub type ProducerSide = (ProducerRx, ProducerTx);
pub type ConsumerSide = (ConsumerRx, ConsumerTx);

const MPSC_BUF_SIZE: usize = 32;

pub fn pipe() -> (ProducerSide, ConsumerSide) {
    let (producer_tx, consumer_rx) = mpsc::channel::<ProducerMessage>(MPSC_BUF_SIZE);
    let (consumer_tx, producer_rx) = mpsc::channel::<ConsumerMessage>(MPSC_BUF_SIZE);

    ((producer_rx, producer_tx), (consumer_rx, consumer_tx))
}
