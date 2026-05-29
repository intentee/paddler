use anyhow::Context as _;
use anyhow::Result;
#[cfg(unix)]
use anyhow::anyhow;
#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(unix)]
use nix::sys::signal::kill;
#[cfg(unix)]
use nix::unistd::Pid;
use tokio::process::Child;

#[cfg(unix)]
pub fn terminate_child(child: &mut Child) -> Result<()> {
    let Some(raw_pid) = child.id() else {
        return Ok(());
    };

    let pid = Pid::from_raw(
        i32::try_from(raw_pid)
            .map_err(|error| anyhow!("PID {raw_pid} does not fit in i32: {error}"))?,
    );

    match kill(pid, Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(errno) => Err(anyhow::Error::new(errno))
            .with_context(|| format!("failed to send SIGTERM to process {raw_pid}")),
    }
}

#[cfg(windows)]
pub fn terminate_child(child: &mut Child) -> Result<()> {
    if child.id().is_none() {
        return Ok(());
    }

    child
        .start_kill()
        .context("failed to terminate child process")
}
