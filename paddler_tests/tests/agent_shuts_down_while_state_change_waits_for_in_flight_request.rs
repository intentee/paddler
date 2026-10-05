#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::desired_state_with_halved_image_resize::desired_state_with_halved_image_resize;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_while_state_change_waits_for_in_flight_request() {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let mut in_flight_stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    in_flight_stream
        .next()
        .await
        .expect("the in-flight request must stream a first token")
        .expect("the message must be readable");

    let initial_desired_state = qwen3_desired_state();

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_halved_image_resize(initial_desired_state)
                .expect("the desired state must be derivable"),
        )
        .await
        .expect("the balancer must accept the desired state");

    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.status.state_application_status != AgentStateApplicationStatus::Applied
            })
        })
        .await
        .expect("the agent must start applying the changed state");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
