use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawn_agent_subprocess::spawn_agent_subprocess;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::subprocess_cluster_error::SubprocessClusterError;

pub fn spawn_signalled_agent_subprocess(
    spawn_agent_subprocess_params: SpawnAgentSubprocessParams,
) -> Result<SignalledSubprocess, SubprocessClusterError> {
    SignalledSubprocess::new(spawn_agent_subprocess(spawn_agent_subprocess_params)?)
}
