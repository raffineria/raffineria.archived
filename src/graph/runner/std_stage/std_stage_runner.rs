use crate::futures::fsm::FSM;
use futures::prelude::*;

use crate::futures::SendBoxedFuture;
use crate::spec::StdStageSpec;
use crate::std_stages::BoxedStdStage;

use super::*;

use graph_channels::{ConsumerChannels, ConsumerChannelsWithResolution};
use graph_channels::{ProducerChannels, ProducerChannelsWithResolution};

pub enum StdStageRunner {
    Init {
        spec: StdStageSpec,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    },
    SpecResolved {
        stage: BoxedStdStage,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    },
    SchemasResolved {
        stage: BoxedStdStage,
        inlets: Vec<ConsumerChannelsWithResolution>,
        outlets: Vec<ProducerChannelsWithResolution>,
    },
    Running {
        inner: SendBoxedFuture<(), StdStageRunnerError>,
    },
}

impl StdStageRunner {
    pub fn new(
        spec: StdStageSpec,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    ) -> Self {
        StdStageRunner::Init {
            spec,
            inlets,
            outlets,
        }
    }
}

impl IntoFuture for StdStageRunner {
    type Future = StdStageRunnerFuture;
    type Item = <StdStageRunnerFuture as Future>::Item;
    type Error = <StdStageRunnerFuture as Future>::Error;

    fn into_future(self) -> Self::Future {
        self.into_fsm_future()
    }
}
