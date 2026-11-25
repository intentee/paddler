use tokio::process::Child;

use crate::signalled_subprocess::SignalledSubprocess;
use crate::subprocess_cluster_error::SubprocessClusterError;

/// # Errors
///
/// Always returns [`SubprocessClusterError::ProcessAlreadyReaped`], standing in for a subprocess reaped before it could be signalled.
pub fn signal_as_already_reaped(
    _subprocess: Child,
) -> Result<SignalledSubprocess, SubprocessClusterError> {
    Err(SubprocessClusterError::ProcessAlreadyReaped)
}
