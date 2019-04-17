use super::substates;

#[derive(Fail, Debug)]
pub enum StageFailure {
    #[fail(display = "StageFailure::Handshake")]
    Handshake(#[cause] substates::HandshakeFailure),

    #[fail(display = "StageFailure::InletFailure[inlet_idx: {}]", inlet_idx)]
    InletFailure {
        inlet_idx: usize,
        #[cause]
        reason: failure::Error,
    },

    #[fail(display = "StageFailure::OutletFailure[outlet_idx: {}]", outlet_idx)]
    OutletFailure {
        outlet_idx: usize,
        #[cause]
        reason: failure::Error,
    },

    #[fail(display = "StageFailure::StageFailure")]
    StageFailure {
        #[cause]
        reason: failure::Error,
    },

    #[fail(display = "StageFailure::RunningFailure")]
    RunningFailure {
        #[cause]
        reason: substates::RunningFailure,
    },
}

impl From<substates::HandshakeFailure> for StageFailure {
    fn from(handshake_failure: substates::HandshakeFailure) -> Self {
        StageFailure::Handshake(handshake_failure)
    }
}

impl From<substates::RunningFailure> for StageFailure {
    fn from(reason: substates::RunningFailure) -> Self {
        StageFailure::RunningFailure { reason }
    }
}
