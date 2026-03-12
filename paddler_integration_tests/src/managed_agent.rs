use std::time::Duration;

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

fn wait_for_child_exit(child: &mut Child) {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break,
        }
    }
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
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if let Some(name) = &params.name {
            command.arg("--name").arg(name);
        }

        let child = command.spawn()?;

        Ok(Self { child })
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.start_kill()?;
        wait_for_child_exit(&mut self.child);

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
