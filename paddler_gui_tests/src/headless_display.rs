use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::Child;
use tokio::process::Command;

pub struct HeadlessDisplay {
    display_name: String,
    xvfb: Child,
}

impl HeadlessDisplay {
    pub async fn start() -> Result<Self> {
        let display_number = std::process::id() % 1000 + 99;
        let display_name = format!(":{display_number}");

        let mut xvfb = Command::new("Xvfb")
            .arg(&display_name)
            .arg("-screen")
            .arg("0")
            .arg("1024x768x24")
            .arg("-nolisten")
            .arg("tcp")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn Xvfb; is it installed in the nix-shell?")?;

        let lock_path = PathBuf::from(format!("/tmp/.X{display_number}-lock"));
        let socket_path = PathBuf::from(format!("/tmp/.X11-unix/X{display_number}"));

        loop {
            if lock_path.exists() && socket_path.exists() {
                break;
            }

            match xvfb.try_wait() {
                Ok(Some(exit_status)) => {
                    bail!("Xvfb exited before becoming ready: {exit_status}");
                }
                Ok(None) => {}
                Err(error) => bail!("failed to check Xvfb status: {error}"),
            }

            tokio::task::yield_now().await;
        }

        Ok(Self { display_name, xvfb })
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl Drop for HeadlessDisplay {
    fn drop(&mut self) {
        if let Some(raw_pid) = self.xvfb.id() {
            #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
            let pid = Pid::from_raw(raw_pid as i32);
            let _ = kill(pid, Signal::SIGTERM);
        }
    }
}
