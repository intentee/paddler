#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::is_unending_generation_text::is_unending_generation_text;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION: u32 = 256;

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

    let Message::Response(ResponseEnvelope {
        response: Response::GeneratedToken(first_token_result),
        ..
    }) = stream
        .next()
        .await
        .expect("the in-flight request must stream a first token")
        .expect("the message must be readable")
    else {
        panic!("the in-flight request must start by streaming a generated token");
    };

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
                ..BalancerDesiredState::default()
            },
        )
        .await
        .expect("the balancer must accept the desired state");

    cluster
        .wait_for_first_agent_issue(|issue| matches!(issue, AgentIssue::ModelFileDoesNotExist(_)))
        .await
        .expect("the agent must report the missing desired model");

    let collected = collect_generated_tokens(stream)
        .await
        .expect("the generated tokens must be collected");
    let streamed_text = format!(
        "{}{}",
        first_token_result.token_text().unwrap_or_default(),
        collected.text
    );

    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );
    assert!(
        is_unending_generation_text(&streamed_text),
        "{streamed_text:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
