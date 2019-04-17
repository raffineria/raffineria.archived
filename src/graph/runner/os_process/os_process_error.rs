use std::io;
use std::process;

use super::*;

#[derive(Fail, Debug)]
pub enum OsProcessError {
    #[fail(display = "OsProcessError::SpawnError")]
    SpawnError(#[cause] spawn::SpawnError),

    #[fail(display = "OsProcessError::ProcessRunError")]
    ProcessRunError(#[cause] io::Error),

    #[fail(display = "OsProcessError::UnexpectedProcessExit")]
    UnexpectedProcessExit(process::ExitStatus),

    #[fail(display = "OsProcessError::HandshakeError")]
    HandshakeError(#[cause] handshake::HandshakeError),

    #[fail(display = "OsProcessError::WireUpError")]
    WireUpError(#[cause] wire_up::WireUpError),
}

impl From<spawn::SpawnError> for OsProcessError {
    fn from(inner: spawn::SpawnError) -> Self {
        OsProcessError::SpawnError(inner)
    }
}

impl From<handshake::HandshakeError> for OsProcessError {
    fn from(inner: handshake::HandshakeError) -> Self {
        OsProcessError::HandshakeError(inner)
    }
}

impl From<wire_up::WireUpError> for OsProcessError {
    fn from(inner: wire_up::WireUpError) -> Self {
        OsProcessError::WireUpError(inner)
    }
}
