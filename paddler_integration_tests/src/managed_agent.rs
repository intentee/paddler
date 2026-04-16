use std::process::ExitStatus;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::Child;
use tokio::time::timeout;

use crate::managed_agent_params::ManagedAgentParams;
use crate::paddler_command;
use crate::terminate_child;

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

    pub fn kill(&mut self) {
        terminate_child(&mut self.child);
    }

    pub async fn sigterm_and_wait_for_exit(
        &mut self,
        graceful_exit_deadline: Duration,
    ) -> Result<ExitStatus> {
        let raw_pid = self
            .child
            .id()
            .ok_or_else(|| anyhow!("agent process has already exited"))?;

        #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
        let pid = Pid::from_raw(raw_pid as i32);

        kill(pid, Signal::SIGTERM)?;

        timeout(graceful_exit_deadline, self.child.wait())
            .await
            .map_err(|_| anyhow!("agent did not exit within {graceful_exit_deadline:?}"))?
            .map_err(Into::into)
    }
}

impl Drop for ManagedAgent {
    fn drop(&mut self) {
        self.kill();
    }
}
