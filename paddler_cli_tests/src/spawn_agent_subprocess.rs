use std::process::Stdio;

use tokio::process::Child;

use crate::paddler_command::paddler_command;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::subprocess_cluster_error::SubprocessClusterError;

pub fn spawn_agent_subprocess(
    SpawnAgentSubprocessParams {
        binary_path,
        management_addr,
        name,
        slots,
    }: SpawnAgentSubprocessParams,
) -> Result<Child, SubprocessClusterError> {
    paddler_command(&binary_path)
        .arg("agent")
        .arg("--management-addr")
        .arg(management_addr.to_string())
        .arg("--name")
        .arg(name)
        .arg("--slots")
        .arg(slots.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(SubprocessClusterError::AgentSpawnFailed)
}
