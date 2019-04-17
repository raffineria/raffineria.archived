use crate::protocol::command::Failure as PortFailure;

#[derive(Debug)]
pub enum ConsumerMessage {
    Pull { max_items: usize },
    Cancel,
}

#[derive(Debug)]
pub enum ProducerMessage {
    Push { items: Vec<Vec<u8>> },
    Complete,
    Fail { failure: PortFailure },
}
