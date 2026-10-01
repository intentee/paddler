use std::io;
use std::num::TryFromIntError;

use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubprocessClusterError {
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
    #[error("The balancer subprocess closed its stdout before announcing its addresses")]
    StdoutClosedBeforeAnnouncement,
    #[error("The balancer subprocess stdout is not piped")]
    StdoutNotPiped,
}
