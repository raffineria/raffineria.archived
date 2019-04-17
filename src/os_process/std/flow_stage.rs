// use futures::prelude::*;

pub struct FlowStage;

// type Flow<In, InErr, Out, OutErr> = Sink<SinkItem = In, SinkError = InErr> + Stream<Item = Out, Error = OutErr>;

// pub struct FlowStage<In, InErr, Out, OutErr, F>
//     where F: Flow<In, InErr, Out, OutErr>
// {
//     flow: F,
// }

// impl<In, InErr, Out, OutErr, F> FlowStage<F>
//     where F: Flow<In, InErr, Out, OutErr>
// {
//     fn from_sink(flow: F) -> Self {
//         Self {
//             flow,
//         }
//     }
// }
