use super::*;
use inlet_wrapper::InletWrapperError;
use outlet_wrapper::OutletWrapperError;

#[derive(Fail, Debug)]
pub enum RunningFailure {
    #[fail(display = "RunningFailure::OutletWrapperError")]
    OutletWrapperError(#[cause] OutletWrapperError),

    #[fail(display = "RunningFailure::InletWrapperError")]
    InletWrapperError(#[cause] InletWrapperError),

    #[fail(display = "RunningFailure::MessageToCommandError")]
    MessageToCommandError(#[cause] MessageToCommandError),

    #[fail(display = "RunningFailure::CommandToMessageError")]
    CommandToMessageError(#[cause] CommandToMessageError),

    #[fail(display = "RunningFailure::ProtocolOutletError")]
    ProtocolOutletError(#[cause] failure::Error),

    #[fail(display = "RunningFailure::ProtocolInletError")]
    ProtocolInletError(#[cause] failure::Error),

    #[fail(display = "RunningFailure::Generic")]
    Generic(#[cause] failure::Error),
}

impl From<OutletWrapperError> for RunningFailure {
    fn from(owe: OutletWrapperError) -> Self {
        RunningFailure::OutletWrapperError(owe)
    }
}

impl From<InletWrapperError> for RunningFailure {
    fn from(iwe: InletWrapperError) -> Self {
        RunningFailure::InletWrapperError(iwe)
    }
}

impl From<MessageToCommandError> for RunningFailure {
    fn from(mtce: MessageToCommandError) -> Self {
        RunningFailure::MessageToCommandError(mtce)
    }
}

impl From<CommandToMessageError> for RunningFailure {
    fn from(ctme: CommandToMessageError) -> Self {
        RunningFailure::CommandToMessageError(ctme)
    }
}
