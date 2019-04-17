use crate::protocol::streams::{inlet_stdin, outlet_stdout};
use crate::protocol::streams::{OwnStdinInlet, OwnStdoutOutlet};

pub struct Ports {
    pub protocol_in: OwnStdinInlet,
    pub protocol_out: OwnStdoutOutlet,
}

impl Ports {
    pub fn stdio() -> Self {
        let protocol_in = inlet_stdin();
        let protocol_out = outlet_stdout();
        Self {
            protocol_in,
            protocol_out,
        }
    }
}
