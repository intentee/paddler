#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;
use std::path::Path;

use futures_util::StreamExt as _;
use futures_util::stream::iter;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::is_unending_generation_text::is_unending_generation_text;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agent_keeps_serving_in_flight_inference_when_the_desired_model_is_missing() {
    let mut cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION,
    )
    .await
    .expect("the cluster must start");
    let mut stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    let first_streamed_item = stream
        .next()
        .await
        .expect("the request must be streaming before the desired model goes missing");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: agent_desired_model_at(Path::new("/nonexistent/model.gguf"))
                    .expect("the missing model path must be absolute"),
                ..BalancerDesiredState::default()
            },
        )
        .await
        .expect("the balancer must accept the desired state");

    cluster
        .wait_for_first_agent_issue(|issue| matches!(issue, AgentIssue::ModelFileDoesNotExist(_)))
        .await
        .expect("the agent must report the missing desired model");

    let collected = collect_generated_tokens(Box::pin(iter([first_streamed_item]).chain(stream)))
        .await
        .expect("the generated tokens must be collected");

    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );
    assert!(
        is_unending_generation_text(&collected.text),
        "{:?}",
        collected.text
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
