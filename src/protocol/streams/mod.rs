mod command_to_message;
mod decoder;
mod encoder;
mod inlet;
mod message_channels;
mod message_to_command;
mod outlet;

pub use decoder::Decoder;
pub use encoder::Encoder;
pub use failure::Error;
pub use inlet::Inlet;
pub use outlet::Outlet;

pub use inlet::{stdin as inlet_stdin, ChildStdoutInlet, OwnStdinInlet};
pub use outlet::{stdout as outlet_stdout, ChildStdinOutlet, OwnStdoutOutlet};

pub use message_channels::{ConsumerRx, ConsumerTx};
pub use message_channels::{ProducerRx, ProducerTx};

pub use command_to_message::{CommandToMessage, CommandToMessageError};
pub use message_to_command::{MessageToCommand, MessageToCommandError};
