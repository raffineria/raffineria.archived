use futures::future;
use futures::prelude::*;
use std::collections::HashMap;

use crate::futures::SendBoxedFuture;
use crate::spec::LogSpec;

use super::*;

pub struct OsProcessRunner {
    cmd: Vec<String>,
    env: HashMap<String, String>,
    log: LogSpec,
    inlets: Vec<graph_channels::ConsumerChannels>,
    outlets: Vec<graph_channels::ProducerChannels>,
}

impl OsProcessRunner {
    pub fn new(
        cmd: Vec<String>,
        env: HashMap<String, String>,
        log: LogSpec,
        inlets: Vec<graph_channels::ConsumerChannels>,
        outlets: Vec<graph_channels::ProducerChannels>,
    ) -> Self {
        trace!("OsProcessRunner::new(...)");

        OsProcessRunner {
            cmd,
            env,
            log,
            inlets,
            outlets,
        }
    }
}

impl IntoFuture for OsProcessRunner {
    type Future = OsProcessRunnerFuture;
    type Item = <OsProcessRunnerFuture as Future>::Item;
    type Error = <OsProcessRunnerFuture as Future>::Error;

    fn into_future(self) -> Self::Future {
        trace!("<OsProcessRunner as IntoFuture>::into_future(...)");

        let spawned = future::result(spawn::spawn(self.cmd, self.env, self.log))
            .map_err(|err| Into::<OsProcessError>::into(err));

        let process_polled = spawned.and_then(|mut spawned| {
            spawned
                .process_handle
                .poll()
                .map_err(|io_err| OsProcessError::ProcessRunError(io_err))
                .and_then(|poll| match poll {
                    Async::Ready(exit_status) => {
                        Err(OsProcessError::UnexpectedProcessExit(exit_status))
                    }
                    Async::NotReady => Ok(spawned),
                })
        });

        let logging_spawned = process_polled.map(|spawned| {
            let _log_capture_spawn = tokio::spawn(
                spawned
                    .log_capture
                    .map(|()| info!("Logging complete"))
                    .map_err(|reason| error!("Logging failure: {:?}", reason)),
            );
            (
                spawned.process_handle,
                spawned.from_process,
                spawned.to_process,
            )
        });

        let inlets = self.inlets;
        let outlets = self.outlets;

        let handshake_done =
            logging_spawned.and_then(move |(process_handle, from_process, to_process)| {
                handshake::handshake(process_handle, from_process, to_process, inlets, outlets)
                    .into_future()
                    .map_err(|err| Into::<OsProcessError>::into(err))
            });

        let stage_complete = handshake_done.and_then(move |handshake_done| {
            let process_handle = handshake_done.process_handle;
            wire_up::wire_up(
                handshake_done.protocol_inlet,
                handshake_done.protocol_outlet,
                handshake_done.inlets_with_resolution,
                handshake_done.outlets_with_resolution,
            )
            .map_err(|err| Into::<OsProcessError>::into(err))
            .and_then(move |()| process_handle.map_err(|err| OsProcessError::ProcessRunError(err)))
        });

        let inner = Box::new(stage_complete.map(|_| ()));

        OsProcessRunnerFuture { inner }
    }
}

pub struct OsProcessRunnerFuture {
    inner: SendBoxedFuture<(), OsProcessError>,
}

impl Future for OsProcessRunnerFuture {
    type Item = ();
    type Error = RunnerError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        trace!("<OsProcessRunnerFuture as Future>::poll(...)");

        self.inner
            .poll()
            .map(|poll| poll.map(|_| ()))
            .map_err(|err| RunnerError::OsProcessError(err))
    }
}
