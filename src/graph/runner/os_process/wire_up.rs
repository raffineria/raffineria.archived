use futures::prelude::*;

use crate::futures::SendBoxedFuture;

use crate::protocol::streams::{ChildStdinOutlet, ChildStdoutInlet};
use crate::protocol::streams::{CommandToMessage, CommandToMessageError};
use crate::protocol::streams::{MessageToCommand, MessageToCommandError};

use super::*;

#[derive(Fail, Debug)]
pub enum WireUpError {
    #[fail(display = "WireUpError::CommandToMessageError")]
    CommandToMessageError(#[cause] CommandToMessageError),

    #[fail(display = "WireUpError::MessageToCommandError")]
    MessageToCommandError(#[cause] MessageToCommandError),

    #[fail(display = "WireUpError::ProtocolOutletError")]
    ProtocolOutletError(#[cause] failure::Error),

    #[fail(display = "WireUpError::ProtocolInletError")]
    ProtocolInletError(#[cause] failure::Error),
}

pub fn wire_up(
    protocol_inlet: ChildStdoutInlet,
    protocol_outlet: ChildStdinOutlet,
    inlets: Vec<graph_channels::ConsumerChannelsWithResolution>,
    outlets: Vec<graph_channels::ProducerChannelsWithResolution>,
) -> SendBoxedFuture<(), WireUpError> {
    let _inlet_schema_resolutions = inlets
        .iter()
        .map(|chans| chans.schema_resolution.clone())
        .collect::<Vec<_>>();
    let _outlet_schema_resolutions = outlets
        .iter()
        .map(|chans| chans.schema_resolution.clone())
        .collect::<Vec<_>>();

    let (inlet_rxs, inlet_txs): (Vec<_>, Vec<_>) =
        inlets.into_iter().map(|chans| (chans.rx, chans.tx)).unzip();

    let (outlet_rxs, outlet_txs): (Vec<_>, Vec<_>) = outlets
        .into_iter()
        .map(|chans| (chans.rx, chans.tx))
        .unzip();

    let protocol_inlet = protocol_inlet.map_err(|err| WireUpError::ProtocolInletError(err));
    let protocol_outlet = protocol_outlet.sink_map_err(|err| WireUpError::ProtocolOutletError(err));

    let command_to_message = CommandToMessage::new(outlet_txs, inlet_txs)
        .sink_map_err(|err| WireUpError::CommandToMessageError(err));
    let message_to_command = MessageToCommand::new(outlet_rxs, inlet_rxs)
        .map_err(|err| WireUpError::MessageToCommandError(err));

    let inlet_bound = command_to_message.send_all(protocol_inlet);
    let outlet_bound = message_to_command.forward(protocol_outlet);

    let fut = inlet_bound.join(outlet_bound).map(|_| ());

    Box::new(fut)
}
