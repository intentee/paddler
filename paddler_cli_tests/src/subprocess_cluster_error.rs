use std::io;
use std::num::TryFromIntError;

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubprocessClusterError {
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
    #[error("The subprocess PID {raw_pid} does not fit into a signal target")]
    ProcessIdOutOfRange {
        raw_pid: u32,
        #[source]
        source: TryFromIntError,
    },
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
    #[error("The agent {agent_name} was reported ready but is missing from the agents snapshot")]
    ReadyAgentMissing { agent_name: String },
    #[error("The balancer subprocess closed its stdout before announcing its addresses")]
    StdoutClosedBeforeAnnouncement,
    #[error("The balancer subprocess stdout is not piped")]
    StdoutNotPiped,
}
