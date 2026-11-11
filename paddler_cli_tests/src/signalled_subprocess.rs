use tokio::process::Child;

use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_process::SubprocessProcess;
use crate::subprocess_signals::SubprocessSignals;

pub struct SignalledSubprocess {
    pub process: SubprocessProcess,
    pub signals: SubprocessSignals,
}

impl SignalledSubprocess {
    /// # Errors
    ///
    /// Returns [`SubprocessClusterError::ProcessAlreadyReaped`] when the child was already reaped, so it can no longer be signalled.
    pub fn new(child: Child) -> Result<Self, SubprocessClusterError> {
        let process = SubprocessProcess::new(child);
        let signals = process
            .signals()
            .ok_or(SubprocessClusterError::ProcessAlreadyReaped)?;

        Ok(Self { process, signals })
    }

    /// # Errors
    ///
    /// Returns [`SubprocessClusterError::AgentSpawnFailed`] when the agent subprocess cannot be spawned, and [`SubprocessClusterError::ProcessAlreadyReaped`] when it was already reaped, so it can no longer be signalled.
    pub fn spawn_agent(
        spawn_agent_subprocess_params: SpawnAgentSubprocessParams,
    ) -> Result<Self, SubprocessClusterError> {
        Self::new(spawn_agent_subprocess_params.spawn()?)
    }
}
