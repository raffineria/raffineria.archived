use futures::prelude::*;

use super::*;

pub enum VertexRunnerFuture {
    Graph(<GraphRunner as IntoFuture>::Future),
    OsProcess(<OsProcessRunner as IntoFuture>::Future),
    StdStage(<StdStageRunner as IntoFuture>::Future),
}

impl Future for VertexRunnerFuture {
    type Item = ();
    type Error = RunnerError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        trace!("<VertexRunnerFuture as Future>::poll(...)");
        match *self {
            VertexRunnerFuture::Graph(ref mut inner) => inner.poll(),
            VertexRunnerFuture::OsProcess(ref mut inner) => inner.poll(),
            VertexRunnerFuture::StdStage(ref mut inner) => inner.poll().map_err(|err| err.into()),
        }
    }
}
