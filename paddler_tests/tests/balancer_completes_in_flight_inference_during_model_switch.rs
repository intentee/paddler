#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_in_flight_inference_during_model_switch() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(1, 1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let expected_output = "the quick brown fox jumps over the lazy dog";

    let stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: format!("root ::= \"{expected_output}\""),
                root: "root".to_owned(),
            }),
            max_tokens: 200,
            raw_prompt: "Say the following: the quick brown fox jumps over the lazy dog".to_owned(),
        })
        .await?;

    // Trigger model switch to a nonexistent path while the request is in flight
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

    assert_eq!(
        collected.text, expected_output,
        "grammar-constrained output must complete despite concurrent model switch"
    );

    cluster.shutdown().await?;

    Ok(())
}
