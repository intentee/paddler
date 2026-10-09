use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::WaitStatus;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;

use crate::subprocess_cluster_error::SubprocessClusterError;

#[derive(Clone, Debug)]
pub struct SubprocessSignals {
    pub kill_requested: Arc<AtomicBool>,
    pub pid: Pid,
}

impl SubprocessSignals {
    pub fn pause(&self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGSTOP)?;

        match waitpid(self.pid, Some(WaitPidFlag::WUNTRACED)) {
            Ok(WaitStatus::Stopped(..)) => Ok(()),
            Ok(status) => Err(SubprocessClusterError::ProcessDidNotStop {
                pid: self.pid,
                status,
            }),
            Err(source) => Err(SubprocessClusterError::ProcessUnobservable {
                pid: self.pid,
                source,
            }),
        }
    }

    pub fn resume(&self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGCONT)
    }

    pub fn kill(&self) -> Result<(), SubprocessClusterError> {
        self.kill_requested.store(true, Ordering::Release);

        self.send(Signal::SIGKILL)
    }

    pub fn terminate(&self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGTERM)
    }

    fn send(&self, signal: Signal) -> Result<(), SubprocessClusterError> {
        kill(self.pid, signal).map_err(|source| SubprocessClusterError::SignalUndeliverable {
            pid: self.pid,
            signal,
            source,
        })
    }
}
