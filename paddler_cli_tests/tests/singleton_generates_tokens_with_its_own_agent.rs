#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_readiness::AgentReadiness;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::snapshots_watcher::SnapshotsWatcher;

const SINGLETON_AGENT_NAME: &str = "singleton-agent";

#[tokio::test(flavor = "multi_thread")]
async fn singleton_generates_tokens_with_its_own_agent() {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let running_singleton = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "singleton",
            "--inference-addr",
            &ephemeral_loopback_addr,
            "--management-addr",
            &ephemeral_loopback_addr,
            "--name",
            SINGLETON_AGENT_NAME,
            "--slots",
            "1",
        ],
        SignalledSubprocess::new,
    )
    .await
    .expect("the singleton must start and announce its addresses")
    .running_balancer;
    let client_management = ClientManagement::new(running_singleton.management_base_url.clone());

    client_management
        .put_balancer_desired_state(CancellationToken::new(), &qwen3_0_6b().into_desired_state())
        .await
        .expect("the singleton must accept its desired state");

    SnapshotsWatcher::of_agents(
        client_management
            .get_agents_stream(CancellationToken::new())
            .await
            .expect("the agents stream must open"),
    )
    .await
    .expect("the agents stream must deliver its first snapshot")
    .wait_for_agent(SINGLETON_AGENT_NAME, &AgentReadiness::SlotsReady(1))
    .await
    .expect("the singleton's own agent must register with its slot ready");

    let collected = collect_generated_tokens(
        ClientInference::new(ClientInferenceParams {
            inference_socket_pool_size: NonZeroUsize::MIN,
            url: running_singleton.inference_base_url.clone(),
        })
        .post_continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the generation request must be accepted"),
    )
    .await
    .expect("the generation must be answered");

    running_singleton
        .shutdown()
        .await
        .expect("the singleton must shut down cleanly");

    assert!(
        collected
            .token_results
            .iter()
            .all(|token_result| token_result.generated_by.as_deref() == Some(SINGLETON_AGENT_NAME))
    );
    collected
        .summary()
        .expect("the singleton's own agent must finish the generation");
}
