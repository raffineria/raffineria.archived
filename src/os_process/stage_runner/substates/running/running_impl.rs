use futures::future;
use futures::prelude::*;

use crate::futures::fsm::FSM;
use crate::os_process::Stage;
use crate::protocol::streams::{OwnStdinInlet, OwnStdoutOutlet};

use super::*;
use inlet_wrapper::InletWrapper;
use outlet_wrapper::OutletWrapper;

impl Running {
    pub fn new<S: Stage>(
        stage: S,
        protocol_out: OwnStdoutOutlet,
        protocol_in: OwnStdinInlet,
    ) -> Self {
        let outlet_schemas = stage.outlets().clone();
        let inlet_schemas = stage.inlets().clone();
        let (outlets, inlets) = stage.into_streams();

        let (consumer_sides, outlets): (Vec<_>, Vec<_>) = outlet_schemas
            .into_iter()
            .zip(outlets.into_iter())
            .map(|(schema, outlet)| OutletWrapper::new(outlet, schema))
            .unzip();

        let (producer_sides, inlets): (Vec<_>, Vec<_>) = inlet_schemas
            .into_iter()
            .zip(inlets.into_iter())
            .map(|(schema, inlet)| InletWrapper::new(inlet, schema))
            .unzip();

        let (producer_rxs, producer_txs): (Vec<_>, Vec<_>) = producer_sides.into_iter().unzip();
        let (consumer_rxs, consumer_txs): (Vec<_>, Vec<_>) = consumer_sides.into_iter().unzip();

        let message_to_command = MessageToCommand::new(producer_rxs, consumer_rxs)
            .map_err(|mtce| Into::<RunningFailure>::into(mtce));
        let command_to_message = CommandToMessage::new(producer_txs, consumer_txs)
            .sink_map_err(|ctme| Into::<RunningFailure>::into(ctme));

        let inbound_commands = protocol_in
            .map_err(|e| RunningFailure::ProtocolInletError(e))
            .forward(command_to_message)
            .map(|(protocol_in_wrapped, _)| protocol_in_wrapped.into_inner());

        let outbound_commands = protocol_out
            .sink_map_err(|e| RunningFailure::ProtocolOutletError(e))
            .send_all(message_to_command)
            .map(|(protocol_out_wrapped, _)| protocol_out_wrapped.into_inner());

        let outlets_done =
            future::join_all(outlets.into_iter().map(|outlet| outlet.into_fsm_future()))
                .map_err(|owe| Into::<RunningFailure>::into(owe));
        let inlets_done = future::join_all(inlets.into_iter().map(|inlet| inlet.into_fsm_future()))
            .map_err(|iwe| Into::<RunningFailure>::into(iwe));

        let done = inbound_commands
            .join(outbound_commands)
            .join(outlets_done)
            .map(|(keep, _)| keep)
            .join(inlets_done)
            .map(|(keep, _)| keep);

        Self {
            inner: Box::new(done),
        }
    }
}
