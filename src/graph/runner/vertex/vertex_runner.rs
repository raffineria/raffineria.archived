use futures::prelude::*;

use crate::spec::RunSpec;

use super::*;

use graph_channels::{ConsumerChannels, ProducerChannels};

pub enum VertexRunner {
    Graph(GraphRunner),
    OsProcess(OsProcessRunner),
    StdStage(StdStageRunner),
}

impl VertexRunner {
    pub fn new(
        run_spec: &RunSpec,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    ) -> Self {
        trace!("VertexRunner::new(...)");

        match *run_spec {
            RunSpec::Graph(ref graph_spec) => {
                VertexRunner::Graph(GraphRunner::new(*graph_spec.clone(), inlets, outlets))
            }

            RunSpec::OsProcess {
                ref cmd,
                ref env,
                ref log,
            } => VertexRunner::OsProcess(OsProcessRunner::new(
                cmd.clone(),
                env.clone(),
                log.clone(),
                inlets,
                outlets,
            )),

            RunSpec::StdStage(ref std_stage_spec) => {
                VertexRunner::StdStage(StdStageRunner::new(std_stage_spec.clone(), inlets, outlets))
            }
        }
    }
}

impl IntoFuture for VertexRunner {
    type Future = VertexRunnerFuture;
    type Item = <VertexRunnerFuture as Future>::Item;
    type Error = <VertexRunnerFuture as Future>::Error;

    fn into_future(self) -> Self::Future {
        trace!("<VertexRunner as IntoFuture>::into_future(...)");
        match self {
            VertexRunner::Graph(graph_runner) => {
                VertexRunnerFuture::Graph(graph_runner.into_future())
            }
            VertexRunner::OsProcess(os_process_runner) => {
                VertexRunnerFuture::OsProcess(os_process_runner.into_future())
            }
            VertexRunner::StdStage(std_stage) => {
                VertexRunnerFuture::StdStage(std_stage.into_future())
            }
        }
    }
}
