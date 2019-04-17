use std::collections::HashMap;

use crate::protocol::Schema;
use crate::spec::{EdgeSpec, PortSpec, VertexSpec};

use super::*;

pub struct VertexPortChannels {
    pub inlets: HashMap<String, Option<ConsumerChannels>>,
    pub outlets: HashMap<String, Option<ProducerChannels>>,
}
impl VertexPortChannels {
    pub fn empty() -> Self {
        VertexPortChannels {
            inlets: HashMap::new(),
            outlets: HashMap::new(),
        }
    }

    pub fn into_chan_maps(
        self,
    ) -> (
        HashMap<String, Option<ConsumerChannels>>,
        HashMap<String, Option<ProducerChannels>>,
    ) {
        (self.inlets, self.outlets)
    }

    pub fn reserve_inlet(mut self, inlet_name: String) -> Result<Self, GraphDefinitionError> {
        match self.inlets.insert(inlet_name.clone(), None) {
            None => Ok(self),
            Some(_) => Err(GraphDefinitionError::DuplicateInletName(inlet_name)),
        }
    }
    pub fn bind_inlet(
        &mut self,
        inlet_name: &str,
        chans: ConsumerChannels,
    ) -> Result<(), GraphDefinitionError> {
        let dest =
            self.inlets
                .get_mut(inlet_name)
                .ok_or(GraphDefinitionError::PortDoesNotExist(
                    inlet_name.to_owned(),
                ))?;
        *dest = Some(chans);
        Ok(())
    }
    pub fn reserve_outlet(mut self, outlet_name: String) -> Result<Self, GraphDefinitionError> {
        match self.outlets.insert(outlet_name.clone(), None) {
            None => Ok(self),
            Some(_) => Err(GraphDefinitionError::DuplicateOutletName(outlet_name)),
        }
    }
    pub fn bind_outlet(
        &mut self,
        outlet_name: &str,
        chans: ProducerChannels,
    ) -> Result<(), GraphDefinitionError> {
        let dest =
            self.outlets
                .get_mut(outlet_name)
                .ok_or(GraphDefinitionError::PortDoesNotExist(
                    outlet_name.to_owned(),
                ))?;
        *dest = Some(chans);
        Ok(())
    }

    pub fn unbound_inlets(&self) -> Vec<String> {
        self.inlets
            .iter()
            .filter_map(|(inlet_name, chan_opt)| {
                if chan_opt.is_none() {
                    Some(inlet_name.to_owned())
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn unbound_outlets(&self) -> Vec<String> {
        self.outlets
            .iter()
            .filter_map(|(outlet_name, chan_opt)| {
                if chan_opt.is_none() {
                    Some(outlet_name.to_owned())
                } else {
                    None
                }
            })
            .collect()
    }
}

pub fn create_port_slots_for_vertices(
    vertex_specs: &HashMap<String, VertexSpec>,
) -> Result<HashMap<String, VertexPortChannels>, RunnerError> {
    vertex_specs
        .iter()
        .map(|(name, vertex_spec)| {
            Ok(VertexPortChannels::empty())
                .and_then(|vps| {
                    vertex_spec
                        .outlets
                        .iter()
                        .fold(Ok(vps), |acc, outlet_name| {
                            acc.and_then(|vps| vps.reserve_outlet(outlet_name.to_owned()))
                        })
                })
                .and_then(|vps| {
                    vertex_spec.inlets.iter().fold(Ok(vps), |acc, outlet_name| {
                        acc.and_then(|vps| vps.reserve_inlet(outlet_name.to_owned()))
                    })
                })
                .map(|vps| (name.to_owned(), vps))
        })
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| err.into())
}

pub fn bind_internal_channels(
    vertex_port_slots: &mut HashMap<String, VertexPortChannels>,
    edge_specs: &Vec<EdgeSpec>,
) -> Result<(), RunnerError> {
    for edge_spec in edge_specs.iter() {
        let schema = Schema::parse(&edge_spec.schema)
            .map_err(|schema_parse_err| GraphDefinitionError::SchemaParseError(schema_parse_err))?;

        let producer_port_spec = &edge_spec.producer;
        let consumer_port_spec = &edge_spec.consumer;

        let (producer_chans, consumer_chans) = graph_channels::pipes(&schema);
        {
            let producer_vps = vertex_port_slots
                .get_mut(&producer_port_spec.vertex)
                .ok_or(GraphDefinitionError::VertexDoesNotExist(
                    producer_port_spec.vertex.to_owned(),
                ))?;
            let () = producer_vps.bind_outlet(&producer_port_spec.port, producer_chans)?;
        }
        {
            let consumer_vps = vertex_port_slots
                .get_mut(&consumer_port_spec.vertex)
                .ok_or(GraphDefinitionError::VertexDoesNotExist(
                    consumer_port_spec.vertex.to_owned(),
                ))?;
            let () = consumer_vps.bind_inlet(&consumer_port_spec.port, consumer_chans)?;
        }
    }
    Ok(())
}

pub fn bind_external_outlets(
    vertex_port_slots: &mut HashMap<String, VertexPortChannels>,
    graph_outlet_port_specs: &Vec<PortSpec>,
    external_producer_channels: Vec<ProducerChannels>,
) -> Result<(), RunnerError> {
    if graph_outlet_port_specs.len() != external_producer_channels.len() {
        Err(RunnerError::GraphDefinitionError(
            GraphDefinitionError::ExternalPortMismatch,
        ))
    } else {
        for (port_spec, chans) in graph_outlet_port_specs
            .iter()
            .zip(external_producer_channels.into_iter())
        {
            let producer_vps = vertex_port_slots.get_mut(&port_spec.vertex).ok_or(
                GraphDefinitionError::VertexDoesNotExist(port_spec.vertex.to_owned()),
            )?;
            let () = producer_vps.bind_outlet(&port_spec.port, chans)?;
        }
        Ok(())
    }
}

pub fn bind_external_inlets(
    vertex_port_slots: &mut HashMap<String, VertexPortChannels>,
    graph_inlet_port_specs: &Vec<PortSpec>,
    external_consumer_channels: Vec<ConsumerChannels>,
) -> Result<(), RunnerError> {
    if graph_inlet_port_specs.len() != external_consumer_channels.len() {
        Err(RunnerError::GraphDefinitionError(
            GraphDefinitionError::ExternalPortMismatch,
        ))
    } else {
        for (port_spec, chans) in graph_inlet_port_specs
            .iter()
            .zip(external_consumer_channels.into_iter())
        {
            let producer_vps = vertex_port_slots.get_mut(&port_spec.vertex).ok_or(
                GraphDefinitionError::VertexDoesNotExist(port_spec.vertex.to_owned()),
            )?;
            let () = producer_vps.bind_inlet(&port_spec.port, chans)?;
        }
        Ok(())
    }
}
