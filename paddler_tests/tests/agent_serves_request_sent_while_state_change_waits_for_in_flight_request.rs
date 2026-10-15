#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::desired_state_with_halved_image_resize::desired_state_with_halved_image_resize;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION: u32 = 512;

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_request_sent_while_state_change_waits_for_in_flight_request() {
    let mut cluster = start_cluster_with_qwen3_and_context_size(
        AgentConfig::uniform(1, 2),
        CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION,
    )
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

    let initial_desired_state = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the balancer must report its desired state");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_halved_image_resize(initial_desired_state),
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

    let (in_flight_collected, sent_during_state_change_collected) = join!(
        collect_generated_tokens(in_flight_stream),
        cluster.continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(4).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
    );

    assert_eq!(
        in_flight_collected
            .expect("the inference request must be accepted")
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );
    sent_during_state_change_collected
        .expect("the request must complete")
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
