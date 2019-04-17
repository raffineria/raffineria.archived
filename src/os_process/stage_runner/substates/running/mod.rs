use futures::prelude::*;

use crate::futures::SendBoxedFuture;
use crate::protocol::streams::{OwnStdinInlet, OwnStdoutOutlet};

use crate::protocol::streams::{CommandToMessage, CommandToMessageError};
use crate::protocol::streams::{MessageToCommand, MessageToCommandError};

mod running_failure;
pub use running_failure::RunningFailure;

mod inlet_wrapper;
mod messages;
mod os_process_channels;
mod outlet_wrapper;
mod running_impl;

type InnerFuture = SendBoxedFuture<(OwnStdinInlet, OwnStdoutOutlet), RunningFailure>;

pub struct Running {
    inner: InnerFuture,
}

impl Future for Running {
    type Item = <InnerFuture as Future>::Item;
    type Error = <InnerFuture as Future>::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}
