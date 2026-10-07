use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::Child;

use crate::subprocess_cluster_error::SubprocessClusterError;

pub fn terminate_child(child: &mut Child) -> Result<()> {
    let Some(raw_pid) = child.id() else {
        return Ok(());
    };

    let pid = Pid::from_raw(
        i32::try_from(raw_pid)
            .map_err(|source| SubprocessClusterError::ProcessIdOutOfRange { raw_pid, source })?,
    );

    match kill(pid, Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(errno) => Err(Error::new(errno))
            .with_context(|| format!("failed to send SIGTERM to process {raw_pid}")),
    }
}
