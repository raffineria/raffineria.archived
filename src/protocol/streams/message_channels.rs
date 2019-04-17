use futures::sync::mpsc;

use crate::protocol::messages::{ConsumerMessage, ProducerMessage};

pub type ProducerTx = mpsc::Sender<ProducerMessage>;
pub type ProducerRx = mpsc::Receiver<ConsumerMessage>;

pub type ConsumerTx = mpsc::Sender<ConsumerMessage>;
pub type ConsumerRx = mpsc::Receiver<ProducerMessage>;
