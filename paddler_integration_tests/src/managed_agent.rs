use anyhow::Result;
use tokio::process::Child;
use tokio::process::Command;

use crate::PADDLER_BINARY_PATH;

pub struct ManagedAgentParams {
    pub management_addr: String,
    pub name: Option<String>,
    pub slots: i32,
}

pub struct ManagedAgent {
    child: Child,
}

impl ManagedAgent {
    pub async fn spawn(params: ManagedAgentParams) -> Result<Self> {
        let mut command = Command::new(PADDLER_BINARY_PATH);

        command
            .arg("agent")
            .arg("--management-addr")
            .arg(&params.management_addr)
            .arg("--slots")
            .arg(params.slots.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(name) = &params.name {
            command.arg("--name").arg(name);
        }

        let child = command.spawn()?;

        Ok(Self { child })
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.start_kill()?;

        Ok(())
    }
}

impl Drop for ManagedAgent {
    fn drop(&mut self) {
        if let Err(error) = self.kill() {
            eprintln!("Failed to kill managed agent: {error}");
        }
    }
}
