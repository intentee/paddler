use anyhow::Result;
use tokio::process::Child;

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
}

impl Drop for ManagedAgent {
    fn drop(&mut self) {
        self.kill();
    }
}
