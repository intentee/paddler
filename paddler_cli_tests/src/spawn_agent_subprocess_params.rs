use std::net::SocketAddr;
use std::process::Stdio;

use tokio::process::Child;

use crate::paddler_command::paddler_command;
use crate::subprocess_cluster_error::SubprocessClusterError;

pub struct SpawnAgentSubprocessParams {
    pub binary_path: String,
    pub management_addr: SocketAddr,
    pub name: String,
    pub slots: u16,
}

impl SpawnAgentSubprocessParams {
    /// # Errors
    ///
    /// Returns [`SubprocessClusterError::AgentSpawnFailed`] when the agent subprocess cannot be spawned.
    pub fn spawn(self) -> Result<Child, SubprocessClusterError> {
        let Self {
            binary_path,
            management_addr,
            name,
            slots,
        } = self;

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
}
