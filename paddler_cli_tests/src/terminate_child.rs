use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use tokio::process::Child;

use crate::child_pid::child_pid;

pub fn terminate_child(child: &mut Child) -> Result<()> {
    let Some(pid) = child_pid(child)? else {
        return Ok(());
    };

    match kill(pid, Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(errno) => Err(Error::new(errno))
            .with_context(|| format!("failed to send SIGTERM to process {pid}")),
    }
}
