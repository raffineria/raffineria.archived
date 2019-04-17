use futures::prelude::*;
use tokio::io::AsyncRead;
use tokio_codec::FramedRead;
use tokio_process::ChildStdout;

use tokio_io::io::AllowStdIo;

use crate::protocol::streams::Decoder;
use crate::protocol::streams::Error;
use crate::protocol::Command;

pub type OwnStdinInlet = Inlet<tokio_stdin_stdout::ThreadedStdin>;
pub type ChildStdoutInlet = Inlet<AllowStdIo<ChildStdout>>;

pub struct Inlet<I: AsyncRead> {
    framed_read: FramedRead<I, Decoder>,
}

impl<I: AsyncRead> std::fmt::Debug for Inlet<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("Inlet"))
    }
}

impl<I: AsyncRead> Inlet<I> {
    pub fn new(source: I) -> Self {
        let decoder = Decoder::new(Command::schema().clone());
        let framed_read = FramedRead::new(source, decoder);

        Self { framed_read }
    }
}

pub fn stdin() -> OwnStdinInlet {
    Inlet::new(tokio_stdin_stdout::stdin(0))
}

impl<I: AsyncRead> Stream for Inlet<I> {
    type Item = Command;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.framed_read
            .poll()
            .map(|async_ok| {
                async_ok.map(|item_opt| {
                    trace!("{:?}.poll() -> item_opt: {:?}", self, item_opt);
                    item_opt
                })
            })
            .map_err(|reason| {
                error!("error: {:?}", reason);
                reason.into()
            })
    }
}
