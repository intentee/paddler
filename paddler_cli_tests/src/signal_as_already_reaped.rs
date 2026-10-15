use tokio::process::Child;

use crate::signalled_subprocess::SignalledSubprocess;
use crate::subprocess_cluster_error::SubprocessClusterError;

pub fn signal_as_already_reaped(
    _subprocess: Child,
) -> Result<SignalledSubprocess, SubprocessClusterError> {
    Err(SubprocessClusterError::ProcessAlreadyReaped)
}
