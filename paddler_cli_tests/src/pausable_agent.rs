use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::running_agent::RunningAgent;

use crate::pausable_agent_params::PausableAgentParams;
use crate::spawn_agent_subprocess::spawn_agent_subprocess;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_process::SubprocessProcess;
use crate::subprocess_signals::SubprocessSignals;

pub struct PausableAgent {
    pub id: String,
    pub signals: SubprocessSignals,
}

impl PausableAgent {
    pub async fn join(
        cluster: &mut Cluster,
        PausableAgentParams {
            binary_path,
            config,
            expected_slots_total,
        }: PausableAgentParams,
    ) -> Result<Self> {
        let child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path,
            management_addr: cluster.balancer.addresses.management,
            name: config.name.clone(),
            slots: config.slot_count,
        })?;
        let signals = SubprocessSignals::of(&child)?;
        let agent_name = config.name.clone();

        cluster.agents.push(RunningAgent::new(
            config,
            Box::new(SubprocessProcess::new(child)),
        ));

        let ready_agent_missing = SubprocessClusterError::ReadyAgentMissing {
            agent_name: agent_name.clone(),
        };
        let id = cluster
            .wait_for_agent_ready(&agent_name, expected_slots_total)
            .await?
            .agents
            .into_iter()
            .find(|registered_agent| registered_agent.name.as_deref() == Some(agent_name.as_str()))
            .map(|registered_agent| registered_agent.id)
            .ok_or(ready_agent_missing)?;

        cluster.agent_ids.push(id.clone());

        Ok(Self { id, signals })
    }
}
