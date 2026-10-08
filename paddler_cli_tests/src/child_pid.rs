use nix::unistd::Pid;
use tokio::process::Child;

use crate::subprocess_cluster_error::SubprocessClusterError;

pub fn child_pid(child: &Child) -> Result<Option<Pid>, SubprocessClusterError> {
    child
        .id()
        .map(|raw_pid| {
            i32::try_from(raw_pid)
                .map(Pid::from_raw)
                .map_err(|source| SubprocessClusterError::ProcessIdOutOfRange { raw_pid, source })
        })
        .transpose()
}
