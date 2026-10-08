use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::WaitStatus;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use tokio::process::Child;

use crate::child_pid::child_pid;
use crate::subprocess_cluster_error::SubprocessClusterError;

#[derive(Clone, Copy, Debug)]
pub struct SubprocessSignals {
    pid: Pid,
}

impl SubprocessSignals {
    pub fn of(child: &Child) -> Result<Self, SubprocessClusterError> {
        let pid = child_pid(child)?.ok_or(SubprocessClusterError::ProcessAlreadyReaped)?;

        Ok(Self { pid })
    }

    pub fn pause(self) -> Result<(), SubprocessClusterError> {
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

    pub fn resume(self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGCONT)
    }

    pub fn kill(self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGKILL)
    }

    fn send(self, signal: Signal) -> Result<(), SubprocessClusterError> {
        kill(self.pid, signal).map_err(|source| SubprocessClusterError::SignalUndeliverable {
            pid: self.pid,
            signal,
            source,
        })
    }
}
