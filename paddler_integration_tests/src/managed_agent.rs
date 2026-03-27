use std::time::Duration;

use anyhow::Result;
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

    pub fn kill(&mut self) {
        terminate_child(&mut self.child);
    }

    pub async fn graceful_shutdown(&mut self, timeout: Duration) -> bool {
        let Some(raw_pid) = self.child.id() else {
            return false;
        };

        #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
        let pid = Pid::from_raw(raw_pid as i32);
        let _ = kill(pid, Signal::SIGTERM);

        match tokio::time::timeout(timeout, self.child.wait()).await {
            Ok(Ok(_exit_status)) => true,
            Ok(Err(wait_error)) => {
                log::error!("Failed to wait for agent process: {wait_error}");

                false
            }
            Err(_timeout_elapsed) => false,
        }
    }
}

impl Drop for ManagedAgent {
    fn drop(&mut self) {
        self.kill();
    }
}
