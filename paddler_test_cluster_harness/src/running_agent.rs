use anyhow::Result;
use tokio::select;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;

use crate::agent_config::AgentConfig;
use crate::agent_readiness::AgentReadiness;
use crate::cluster_harness_error::ClusterHarnessError;
use crate::managed_process::ManagedProcess;
use crate::snapshots_watcher::SnapshotsWatcher;

pub struct RunningAgent {
    pub config: AgentConfig,
    process: Box<dyn ManagedProcess>,
}

impl RunningAgent {
    #[must_use]
    pub const fn new(config: AgentConfig, process: Box<dyn ManagedProcess>) -> Self {
        Self { config, process }
    }

    pub async fn wait_until_ready(
        &mut self,
        agents_watcher: &mut SnapshotsWatcher<AgentControllerPoolSnapshot>,
        readiness: AgentReadiness,
    ) -> Result<AgentControllerSnapshot, ClusterHarnessError> {
        select! {
            ready_agent = agents_watcher.wait_for_agent(&self.config.name, &readiness) => ready_agent,
            process_end = self.process.exited() => Err(ClusterHarnessError::AgentExitedBeforeReady {
                agent_name: self.config.name.clone(),
                process_end,
            }),
        }
    }

    pub async fn shutdown(self) -> Result<()> {
        self.process.shutdown().await
    }
}
