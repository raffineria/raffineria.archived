use futures::prelude::*;
use tokio::io::AsyncWrite;
use tokio_codec::FramedWrite;
use tokio_io::io::AllowStdIo;
use tokio_process::ChildStdin;

use crate::protocol::streams::Encoder;
use crate::protocol::streams::Error;
use crate::protocol::Command;

pub type OwnStdoutOutlet = Outlet<tokio_stdin_stdout::ThreadedStdout>;
pub type ChildStdinOutlet = Outlet<AllowStdIo<ChildStdin>>;

pub struct Outlet<O: AsyncWrite> {
    framed_write: FramedWrite<O, Encoder>,
}

impl<O: AsyncWrite> std::fmt::Debug for Outlet<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("Outlet"))
    }
}

impl<O: AsyncWrite> Outlet<O> {
    pub fn new(sink: O) -> Self {
        let encoder = Encoder::new(Command::schema().clone());
        let framed_write = FramedWrite::new(sink, encoder);

        Self { framed_write }
    }
}

pub fn stdout() -> OwnStdoutOutlet {
    Outlet::new(tokio_stdin_stdout::stdout(0))
}

impl<O: AsyncWrite> Sink for Outlet<O> {
    type SinkItem = Command;
    type SinkError = Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        trace!("{:?}.start_send({:?})", self, item);
        self.framed_write.start_send(item).map_err(|reason| {
            warn!("error: {:?}", reason);
            reason.into()
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed_write
            .poll_complete()
            .map_err(|reason| reason.into())
    }
}
