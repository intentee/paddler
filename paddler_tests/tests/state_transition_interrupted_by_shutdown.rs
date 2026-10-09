#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

pub struct StateTransitionInterruptedByShutdown {
    pub initial_desired_state: BalancerDesiredState,
    pub next_desired_state: BalancerDesiredState,
    pub slot_count: u16,
}

impl StateTransitionInterruptedByShutdown {
    pub async fn shut_down_while_the_agent_applies_the_next_state(self) {
        let mut cluster = start_cluster(ClusterParams {
            agents: vec![AgentConfig::single(self.slot_count)],
            desired_state: ClusterDesiredState::Apply(Box::new(self.initial_desired_state)),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        })
        .await
        .expect("the cluster must start");
        let agent_id = cluster
            .agent_ids
            .first()
            .expect("the cluster must register its agent")
            .clone();

        cluster
            .agents_watcher
            .until_agent(&agent_id, |snapshot| {
                snapshot.agents.iter().all(|agent| {
                    agent.status.state_application_status == AgentStateApplicationStatus::Applied
                })
            })
            .await
            .expect("the agent must apply the initial state");
        cluster
            .client_management
            .put_balancer_desired_state(CancellationToken::new(), &self.next_desired_state)
            .await
            .expect("the balancer must accept the next state");
        cluster
            .agents_watcher
            .until_agent(&agent_id, |snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent.status.state_application_status != AgentStateApplicationStatus::Applied
                })
            })
            .await
            .expect("the agent must start applying the next state");

        cluster
            .shutdown()
            .await
            .expect("the cluster must shut down cleanly while its agent applies the next state");
    }
}
