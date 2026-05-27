#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::inference_client::Response as InferenceResponse;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_in_flight_inference_during_model_switch() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let expected_output = "the quick brown fox jumps over the lazy dog";

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: format!("root ::= \"{expected_output}\""),
                root: "root".to_owned(),
            }),
            max_tokens: 200,
            raw_prompt: "Say the following: the quick brown fox jumps over the lazy dog".to_owned(),
        })
        .await?;

    // Wait for the first generated-token message before triggering the model
    // switch. This guarantees the agent has acquired its inference slot and
    // entered the generating phase, so the agent's `drain_in_flight_requests`
    // correctly waits for the in-flight request to finish before tearing
    // down the arbiter. Without this wait, the model-switch can race the
    // request through the scheduler queue: drain sees zero slots in use,
    // returns immediately, the arbiter is shut down, and the queued request
    // times out with no scheduler to process it.
    let mut buffered_text = String::new();
    loop {
        let next = stream
            .next()
            .await
            .ok_or_else(|| anyhow!("inference stream ended before producing any token"))??;
        if let InferenceMessage::Response(envelope) = next
            && let InferenceResponse::GeneratedToken(token_result) = envelope.response
            && let Some(token_text) = token_result.token_text()
        {
            buffered_text.push_str(token_text);
            break;
        }
    }

    let switch_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    cluster
        .paddler_client
        .management()
        .put_balancer_desired_state(&switch_state)
        .await
        .map_err(anyhow::Error::new)?;

    let collected = collect_generated_tokens(stream).await?;

    let mut full_text = buffered_text;
    full_text.push_str(&collected.text);

    assert_eq!(
        full_text, expected_output,
        "grammar-constrained output must complete despite concurrent model switch"
    );

    cluster.shutdown().await?;

    Ok(())
}
