use futures::prelude::*;

pub type SendBoxedFuture<Item, Error> =
    Box<dyn Future<Item = Item, Error = Error> + Send + 'static>;
pub type SendBoxedStream<Item, Error> =
    Box<dyn Stream<Item = Item, Error = Error> + Send + 'static>;
pub type SendBoxedSink<Item, Error> =
    Box<dyn Sink<SinkItem = Item, SinkError = Error> + Send + 'static>;

// pub type BoxedFuture<Item, Error> = Box<dyn Future<Item = Item, Error = Error> + 'static>;
// pub type BoxedStream<Item, Error> = Box<dyn Stream<Item = Item, Error = Error> + 'static>;
// pub type BoxedSink<Item, Error> =
//     Box<dyn Sink<SinkItem = Item, SinkError = Error> + 'static>;
