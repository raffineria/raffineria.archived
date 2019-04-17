use std::collections::HashMap;

use std::io;
use std::process;

use futures::future;
use futures::prelude::*;

use tokio::fs::OpenOptions;
use tokio_codec::{FramedRead, FramedWrite, LinesCodec};
use tokio_io::io::AllowStdIo;
use tokio_process::CommandExt;

use crate::futures::SendBoxedFuture;
use crate::protocol::streams::{ChildStdinOutlet, ChildStdoutInlet};
use crate::spec::LogSpec;

pub type ProcessHandle = tokio_process::Child;

#[derive(Fail, Debug)]
pub enum SpawnError {
    #[fail(display = "SpawnError::EmptyCmd")]
    EmptyCmd,

    #[fail(display = "SpawnError::LogOpenError")]
    LogOpenError(#[cause] io::Error),

    #[fail(display = "SpawnError::StderrReadError")]
    StderrReadError(#[cause] io::Error),

    #[fail(display = "SpawnError::LogWriteError")]
    LogWriteError(#[cause] io::Error),

    #[fail(display = "SpawnError::ProcessStartError")]
    ProcessStartError(#[cause] io::Error),

    #[fail(display = "SpawnError::StdinMissing")]
    StdinMissing,

    #[fail(display = "SpawnError::StdoutMissing")]
    StdoutMissing,

    #[fail(display = "SpawnError::StderrMissing")]
    StderrMissing,
    // #[fail(display = "SpawnError::ProcessRunError")]
    // ProcessRunError(#[cause] io::Error),

    // #[fail(display = "SpawnError::ProcessTerminated: {:?}", _0)]
    // ProcessTerminated(process::ExitStatus),
}

pub struct Spawned {
    pub from_process: ChildStdoutInlet,
    pub to_process: ChildStdinOutlet,
    pub process_handle: ProcessHandle,
    pub log_capture: SendBoxedFuture<(), SpawnError>,
}

pub fn spawn(
    cmd: Vec<String>,
    env: HashMap<String, String>,
    log_spec: LogSpec,
) -> Result<Spawned, SpawnError> {
    trace!(
        "spawn(cmd: {:?}; env: {:?}; log_spec: {:?})",
        cmd,
        env,
        log_spec
    );

    let mut command = process::Command::new(cmd.get(0).ok_or(SpawnError::EmptyCmd)?.to_owned());

    command
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped());

    if should_capture_log(&log_spec) {
        command.stderr(process::Stdio::piped());
    }

    for arg in &cmd[1..] {
        command.arg(arg);
    }

    for (key, value) in env.into_iter() {
        command.env(key, value);
    }

    let process_handle_result: Result<ProcessHandle, io::Error> = command.spawn_async();
    let mut process_handle: ProcessHandle =
        process_handle_result.map_err(|io_err| SpawnError::ProcessStartError(io_err))?;

    let stdin_writer = process_handle
        .stdin()
        .take()
        .ok_or(SpawnError::StdinMissing)?;
    let stdout_reader = process_handle
        .stdout()
        .take()
        .ok_or(SpawnError::StdoutMissing)?;

    let log_capture = create_log_capture(&mut process_handle, log_spec);

    let from_process = ChildStdoutInlet::new(AllowStdIo::new(stdout_reader));
    let to_process = ChildStdinOutlet::new(AllowStdIo::new(stdin_writer));

    Ok(Spawned {
        process_handle,
        from_process,
        to_process,
        log_capture,
    })
}

fn should_capture_log(log_spec: &LogSpec) -> bool {
    LogSpec::NoCapture != *log_spec
}

fn create_log_capture(
    process_handle: &mut ProcessHandle,
    log_spec: LogSpec,
) -> SendBoxedFuture<(), SpawnError> {
    let mut take_stderr = move || {
        process_handle
            .stderr()
            .take()
            .ok_or(SpawnError::StderrMissing)
            .map(|stderr_reader| AllowStdIo::new(stderr_reader))
            .map(|async_stderr_reader| FramedRead::new(async_stderr_reader, LinesCodec::new()))
            .map(|framed_read| framed_read.map_err(|io_err| SpawnError::StderrReadError(io_err)))
            .into_future()
    };

    match log_spec {
        LogSpec::NoCapture => Box::new(future::ok(())),

        LogSpec::Null => Box::new(
            take_stderr()
                .and_then(|stderr_framed_read| stderr_framed_read.forward(LogSinkNull))
                .map(|(_, _)| ()),
        ),

        LogSpec::File { path } => Box::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|io_err| SpawnError::LogOpenError(io_err))
                .map(|file| {
                    FramedWrite::new(file, LinesCodec::new())
                        .sink_map_err(|io_err| SpawnError::LogWriteError(io_err))
                })
                .join(take_stderr())
                .and_then(|(framed_write, stderr_framed_read)| {
                    stderr_framed_read.forward(framed_write)
                })
                .map(|(_, _)| ()),
        ),
    }
}

struct LogSinkNull;

impl Sink for LogSinkNull {
    type SinkItem = String;
    type SinkError = SpawnError;

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
    fn start_send(&mut self, _log_line: String) -> StartSend<Self::SinkItem, Self::SinkError> {
        Ok(AsyncSink::Ready)
    }
}
