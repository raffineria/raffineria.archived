use super::{OwnStdinInlet, OwnStdoutOutlet, Stage, StageRunner};

impl<S: Stage> StageRunner<S> {
    pub fn new(stage: S, protocol_in: OwnStdinInlet, protocol_out: OwnStdoutOutlet) -> Self {
        StageRunner::Init {
            stage,
            protocol_out,
            protocol_in,
        }
    }
}
