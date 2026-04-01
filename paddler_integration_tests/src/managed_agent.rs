use std::process::ExitStatus;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::Child;

use crate::paddler_command;
use crate::terminate_child;

pub struct ManagedAgentParams {
    pub management_addr: String,
    pub name: Option<String>,
    pub slots: i32,
}

pub struct ManagedAgent {
    child: Child,
}

impl ManagedAgent {
    pub fn spawn(params: &ManagedAgentParams) -> Result<Self> {
        let mut command = paddler_command();

        command
            .arg("agent")
            .arg("--management-addr")
            .arg(&params.management_addr)
            .arg("--slots")
            .arg(params.slots.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if let Some(name) = &params.name {
            command.arg("--name").arg(name);
        }

        let child = command.spawn()?;

        Ok(Self { child })
    }

    pub async fn graceful_shutdown(&mut self) -> Result<ExitStatus> {
        let raw_pid = self
            .child
            .id()
            .ok_or_else(|| anyhow!("Agent process already exited"))?;

        #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
        let pid = Pid::from_raw(raw_pid as i32);

        kill(pid, Signal::SIGTERM)?;

        let exit_status = tokio::time::timeout(
            Duration::from_secs(10),
            self.child.wait(),
        )
        .await
        .map_err(|timeout_error| anyhow!("Agent did not exit within 10 seconds after SIGTERM: {timeout_error}"))??;

        Ok(exit_status)
    }

    pub fn kill(&mut self) {
        terminate_child(&mut self.child);
    }
}

impl Drop for ManagedAgent {
    fn drop(&mut self) {
        self.kill();
    }
}
