use futures::future;
use futures::prelude::*;

use crate::futures::fsm::*;

use super::*;

impl FSM for GraphRunner {
    type Item = ();
    type Error = RunnerError;

    fn turn(self) -> TurnResult<Self> {
        match self {
            GraphRunner::Init {
                graph_spec,
                inlets,
                outlets,
            } => {
                trace!("<GraphRunner::Init as FSM>::turn(...)");

                let mut vertex_port_chans =
                    port_bind_utils::create_port_slots_for_vertices(&graph_spec.vertices)?;

                let () = port_bind_utils::bind_internal_channels(
                    &mut vertex_port_chans,
                    &graph_spec.edges,
                )?;

                let () = port_bind_utils::bind_external_outlets(
                    &mut vertex_port_chans,
                    &graph_spec.outlets,
                    outlets,
                )?;

                let () = port_bind_utils::bind_external_inlets(
                    &mut vertex_port_chans,
                    &graph_spec.inlets,
                    inlets,
                )?;

                for (vertex_name, port_slots) in vertex_port_chans.iter() {
                    let unbound_inlets = port_slots.unbound_inlets();
                    let unbound_outlets = port_slots.unbound_outlets();

                    if !unbound_inlets.is_empty() || !unbound_outlets.is_empty() {
                        Err(GraphDefinitionError::UnboundPorts {
                            vertex: vertex_name.to_owned(),
                            inlets: unbound_inlets,
                            outlets: unbound_outlets,
                        })?;
                    }
                }

                Ok(TurnOk::PollMore(GraphRunner::StartVertices {
                    graph_spec,
                    vertex_port_chans,
                }))
            }

            GraphRunner::StartVertices {
                graph_spec,
                mut vertex_port_chans,
            } => {
                trace!("<GraphRunner::StartVertices as FSM>::turn(...)");

                let vertex_runners = graph_spec
                    .vertices
                    .iter()
                    .map(|(vertex_name, vertex_spec)| {
                        let chans = vertex_port_chans
                            .remove(vertex_name)
                            .expect("Missing channels-set vertex");
                        let (mut in_chan_map, mut out_chan_map) = chans.into_chan_maps();
                        let in_chans = vertex_spec
                            .inlets
                            .iter()
                            .map(|inlet_name| {
                                in_chan_map
                                    .remove(inlet_name)
                                    .expect("Missing inlet-channel of vertex")
                                    .expect("Inlet channel remained unbound")
                            })
                            .collect::<Vec<_>>();
                        let out_chans = vertex_spec
                            .outlets
                            .iter()
                            .map(|outlet_name| {
                                out_chan_map
                                    .remove(outlet_name)
                                    .expect("Missing outlet-channel of vertex")
                                    .expect("Outlet channel remained unbound")
                            })
                            .collect::<Vec<_>>();

                        assert!(in_chan_map.is_empty());
                        assert!(out_chan_map.is_empty());

                        (vertex_name, vertex_spec, in_chans, out_chans)
                    })
                    .map(|(_vertex_name, vertex_spec, in_chans, out_chans)| {
                        VertexRunner::new(&vertex_spec.run, in_chans, out_chans)
                    })
                    .collect::<Vec<_>>();

                let vertices_joined_future = future::join_all(vertex_runners);

                Ok(TurnOk::PollMore(GraphRunner::Running(
                    vertices_joined_future,
                )))
            }

            GraphRunner::Running(mut joined) => {
                trace!("<GraphRunner::Running as FSM>::turn(...)");

                joined.poll().map(|poll| match poll {
                    Async::NotReady => TurnOk::Suspend(GraphRunner::Running(joined)),
                    Async::Ready(_) => TurnOk::Ready(()),
                })
            }
        }
    }
}
