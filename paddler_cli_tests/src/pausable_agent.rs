use anyhow::Result;

use paddler_test_cluster_harness::agent_readiness::AgentReadiness;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::running_agent::RunningAgent;

use crate::pausable_agent_params::PausableAgentParams;
use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::spawn_signalled_agent_subprocess::spawn_signalled_agent_subprocess;
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
            management_addr,
        }: PausableAgentParams,
    ) -> Result<Self> {
        let SignalledSubprocess {
            process: agent_process,
            signals,
        } = spawn_signalled_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path,
            management_addr,
            name: config.name.clone(),
            slots: config.slot_count,
        })?;
        let mut running_agent = RunningAgent::new(config, Box::new(agent_process));
        let ready_agent = running_agent
            .wait_until_ready(
                &mut cluster.agents_watcher,
                AgentReadiness::SlotsReady(expected_slots_total),
            )
            .await?;

        cluster.agent_ids.push(ready_agent.id.clone());
        cluster.agents.push(running_agent);

        Ok(Self {
            id: ready_agent.id,
            signals,
        })
    }
}
