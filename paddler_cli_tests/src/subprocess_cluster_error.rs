use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubprocessClusterError {
    #[error(
        "The balancer subprocess announcement is not valid balancer addresses: {announcement:?}"
    )]
    AnnouncementInvalid {
        announcement: String,
        #[source]
        source: serde_json::Error,
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
