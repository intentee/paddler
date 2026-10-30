use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

const REPEATING_AGENT_ID: &str = "repeating-agent";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_keeps_an_agent_that_repeats_its_status_registered() {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");
    let status_report = |desired_slots_total: u16, version: u64| SlotAggregatedStatusSnapshot {
        status: AgentStatus {
            desired_slots_total,
            ..AgentStatus::default()
        },
        version,
    };
    let mut repeating_agent = UntrustedManagementAgent::connect_registered(
        cluster.balancer.addresses.management,
        REPEATING_AGENT_ID,
        status_report(1, 1),
    )
    .await
    .expect("the repeating agent must register");
    repeating_agent
        .report_status(status_report(1, 2))
        .await
        .expect("the repeated status must be sent");
    repeating_agent
        .report_status(status_report(2, 3))
        .await
        .expect("the changed status must be sent");

    cluster
        .agents_watcher
        .until(|snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == REPEATING_AGENT_ID && agent.status.desired_slots_total == 2
            })
        })
        .await
        .expect("the balancer must keep the agent and take its changed status");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
