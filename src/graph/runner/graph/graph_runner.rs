use futures::future;

use crate::spec::GraphSpec;
use std::collections::HashMap;

use super::*;

use port_bind_utils::VertexPortChannels;

pub enum GraphRunner {
    Init {
        graph_spec: GraphSpec,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    },

    StartVertices {
        graph_spec: GraphSpec,
        vertex_port_chans: HashMap<String, VertexPortChannels>,
    },

    Running(future::JoinAll<Vec<VertexRunner>>),
}

impl GraphRunner {
    pub fn top_level(graph_spec: GraphSpec) -> Self {
        Self::new(graph_spec, Vec::new(), Vec::new())
    }

    pub fn new(
        graph_spec: GraphSpec,
        inlets: Vec<ConsumerChannels>,
        outlets: Vec<ProducerChannels>,
    ) -> Self {
        trace!("GraphRunner::new(...)");
        GraphRunner::Init {
            graph_spec,
            inlets,
            outlets,
        }
    }
}
