use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use tokio::process::Child;

use crate::paddler_command::paddler_command;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;

pub fn spawn_agent_subprocess(
    SpawnAgentSubprocessParams {
        management_addr,
        name,
        slots,
    }: SpawnAgentSubprocessParams,
) -> Result<Child> {
    let mut command = paddler_command();

    command
        .arg("agent")
        .arg("--management-addr")
        .arg(management_addr.to_string())
        .arg("--slots")
        .arg(slots.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(agent_name) = name {
        command.arg("--name").arg(agent_name);
    }

    command
        .spawn()
        .context("failed to spawn paddler agent subprocess")
}
