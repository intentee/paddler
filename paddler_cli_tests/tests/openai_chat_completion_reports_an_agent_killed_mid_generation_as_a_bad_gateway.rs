#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use tokio::spawn;

use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::openai_chat_completion_failure_status::openai_chat_completion_failure_status;

const TEXT_GENERATION_SLOTS: u16 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_an_agent_killed_mid_generation_as_a_bad_gateway() {
    let PausableAgentCluster {
        mut cluster,
        pausable_agent,
    } = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        expected_slots_total: TEXT_GENERATION_SLOTS,
        slot_count: TEXT_GENERATION_SLOTS,
    })
    .await
    .expect("a text generation cluster must start");

    pausable_agent
        .signals
        .pause()
        .expect("the agent must stop before the generation reaches it");

    let chat_completion = spawn(openai_chat_completion_failure_status(&cluster));

    cluster
        .wait_for_slots_processing(&pausable_agent.id, 1)
        .await
        .expect("the balancer must dispatch the generation to the paused agent");
    pausable_agent
        .signals
        .kill()
        .expect("the agent must be killed while it holds the generation");

    assert_eq!(
        chat_completion
            .await
            .expect("the chat completion task must not panic")
            .expect("the chat completion must fail with an OpenAI error body"),
        StatusCode::BAD_GATEWAY
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
