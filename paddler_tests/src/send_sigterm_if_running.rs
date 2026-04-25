use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::Child;

pub fn send_sigterm_if_running(child: &Child) -> Result<()> {
    let Some(raw_pid) = child.id() else {
        return Ok(());
    };

    let pid = Pid::from_raw(
        i32::try_from(raw_pid).map_err(|error| anyhow!("PID {raw_pid} does not fit in i32: {error}"))?,
    );

    match kill(pid, Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(errno) => Err(anyhow::Error::new(errno))
            .with_context(|| format!("failed to send SIGTERM to process {raw_pid}")),
    }
}
