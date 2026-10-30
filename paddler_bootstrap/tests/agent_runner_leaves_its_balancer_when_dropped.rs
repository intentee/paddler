use tokio_util::sync::CancellationToken;
use url::Url;

use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_client::client_management::ClientManagement;
use paddler_test_cluster_harness::snapshots_watcher::SnapshotsWatcher;

use crate::agent_runner_params_for_balancer_at::agent_runner_params_for_balancer_at;
use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test(flavor = "multi_thread")]
async fn agent_runner_leaves_its_balancer_when_dropped() {
    let balancer_runner =
        BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
            .await
            .expect("a runner on ephemeral ports must start");
    let mut agents_watcher = SnapshotsWatcher::of_agents(
        ClientManagement::new(
            Url::parse(&format!("http://{}", balancer_runner.addresses.management))
                .expect("the management address must form a URL"),
        )
        .get_agents_stream(CancellationToken::new())
        .await
        .expect("the agents stream must open"),
    )
    .await
    .expect("the agents stream must deliver its first snapshot");
    let agent_runner = AgentRunner::start(agent_runner_params_for_balancer_at(
        CancellationToken::new(),
        balancer_runner.addresses.management.to_string(),
    ));

    agents_watcher
        .until(|snapshot| snapshot.agents.len() == 1)
        .await
        .expect("the agent must register with its balancer");

    drop(agent_runner);

    agents_watcher
        .until(|snapshot| snapshot.agents.is_empty())
        .await
        .expect("a dropped agent runner must leave its balancer");
}
