use futures::sync::mpsc;
use futures::sync::mpsc::{Receiver, Sender};

use crate::protocol::messages::{ConsumerMessage, ProducerMessage};
use crate::protocol::{Schema, SchemaResolution, SchemaResolutionError};

const MPSC_BUFFER_SIZE: usize = 32;

pub type ProducerChannels = Channels<ConsumerMessage, ProducerMessage>;
pub type ConsumerChannels = Channels<ProducerMessage, ConsumerMessage>;
pub type ProducerChannelsWithResolution = ChannelsWithResolution<ConsumerMessage, ProducerMessage>;
pub type ConsumerChannelsWithResolution = ChannelsWithResolution<ProducerMessage, ConsumerMessage>;

#[derive(Debug)]
pub struct Channels<In, Out> {
    pub schema: Schema,
    pub rx: Receiver<In>,
    pub tx: Sender<Out>,
}

impl<In, Out> Channels<In, Out> {
    pub fn new(schema: &Schema, rx: Receiver<In>, tx: Sender<Out>) -> Self {
        Self {
            schema: schema.clone(),
            rx,
            tx,
        }
    }
}

impl ConsumerChannels {
    pub fn resolve_with_reader_schema(
        self,
        reader_schema: &Schema,
    ) -> Result<ChannelsWithResolution<ProducerMessage, ConsumerMessage>, SchemaResolutionError>
    {
        ChannelsWithResolution::new(&self.schema, reader_schema, self.rx, self.tx)
    }
}
impl ProducerChannels {
    pub fn resolve_with_writer_schema(
        self,
        writer_schema: &Schema,
    ) -> Result<ChannelsWithResolution<ConsumerMessage, ProducerMessage>, SchemaResolutionError>
    {
        ChannelsWithResolution::new(writer_schema, &self.schema, self.rx, self.tx)
    }
}

#[derive(Debug)]
pub struct ChannelsWithResolution<In, Out> {
    pub reader_schema: Schema,
    pub writer_schema: Schema,
    pub schema_resolution: SchemaResolution,
    pub rx: Receiver<In>,
    pub tx: Sender<Out>,
}

impl<In, Out> ChannelsWithResolution<In, Out> {
    pub fn new(
        writer_schema: &Schema,
        reader_schema: &Schema,
        rx: Receiver<In>,
        tx: Sender<Out>,
    ) -> Result<Self, SchemaResolutionError> {
        SchemaResolution::new(writer_schema, reader_schema).map(move |schema_resolution| Self {
            schema_resolution,
            writer_schema: writer_schema.clone(),
            reader_schema: reader_schema.clone(),
            rx,
            tx,
        })
    }
}

pub fn pipes(schema: &Schema) -> (ProducerChannels, ConsumerChannels) {
    let (producer_tx, consumer_rx) = mpsc::channel(MPSC_BUFFER_SIZE);
    let (consumer_tx, producer_rx) = mpsc::channel(MPSC_BUFFER_SIZE);

    (
        ProducerChannels::new(schema, producer_rx, producer_tx),
        ConsumerChannels::new(schema, consumer_rx, consumer_tx),
    )
}
