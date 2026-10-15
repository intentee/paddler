#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;
use tokio::spawn;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::minimal_openai_chat_completion::minimal_openai_chat_completion;

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

    let chat_completion = spawn(minimal_openai_chat_completion(
        cluster
            .balancer
            .compat_openai_url(OpenAIApiPath::CHAT_COMPLETIONS)
            .expect("the balancer must serve the OpenAI compatibility service"),
    ));

    cluster
        .wait_for_slots_processing(&pausable_agent.id, 1)
        .await
        .expect("the balancer must dispatch the generation to the paused agent");
    pausable_agent
        .signals
        .kill()
        .expect("the agent must be killed while it holds the generation");

    let response = chat_completion
        .await
        .expect("the chat completion task must not panic")
        .expect("the balancer must answer the chat completion");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    OpenAIValidator::new()
        .expect("the OpenAI specification must load")
        .validate_error_response(
            &response
                .json::<Value>()
                .await
                .expect("the error body must be JSON"),
        )
        .expect("the error body must be an OpenAI error");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
