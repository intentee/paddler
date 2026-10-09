use std::io;
use std::process::ExitStatus;

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubprocessClusterError {
    #[error("The balancer subprocess exited with {exit_status} before announcing its addresses")]
    BalancerExitedBeforeAnnouncing { exit_status: ExitStatus },
    #[error("The subprocess {pid} reported {status:?} instead of stopping")]
    ProcessDidNotStop { pid: Pid, status: WaitStatus },
    #[error("The subprocess was already reaped, so it can no longer be signalled")]
    ProcessAlreadyReaped,
    #[error("Unable to observe the state of subprocess {pid}")]
    ProcessUnobservable {
        pid: Pid,
        #[source]
        source: Errno,
    },
    #[error("Unable to send {signal} to subprocess {pid}")]
    SignalUndeliverable {
        pid: Pid,
        signal: Signal,
        #[source]
        source: Errno,
    },
    #[error("The subprocess exited with {exit_status}")]
    ProcessExitedWithFailure { exit_status: ExitStatus },
    #[error("Unable to observe how the subprocess exited")]
    ProcessExitUnobservable(#[source] io::Error),
    #[error(
        "The balancer subprocess announcement is not valid balancer addresses: {announcement:?}"
    )]
    AnnouncementInvalid {
        announcement: String,
        #[source]
        source: SerdeJsonError,
    },
    #[error("Unable to read the balancer subprocess announcement")]
    AnnouncementUnreadable {
        #[source]
        source: io::Error,
    },
    #[error("The balancer subprocess stdout is not piped")]
    StdoutNotPiped,
}
