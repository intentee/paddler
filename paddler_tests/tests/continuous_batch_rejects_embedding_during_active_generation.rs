#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::qwen3_desired_state_with_embeddings::qwen3_desired_state_with_embeddings;
use paddler_tests::start_cluster_without_model::start_cluster_without_model;
use tokio_util::sync::CancellationToken;

const NEVER_COMPLETING_GRAMMAR: &str = r#"root ::= "apple " root"#;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_embedding_during_active_generation() -> Result<()> {
    let mut cluster = start_cluster_without_model(
        AgentConfig::uniform(1, 2),
        InferenceParameters::deterministic(),
    )
    .await?;

    let generation_cancellation = CancellationToken::new();
    let mut generation_stream = cluster
        .continue_from_raw_prompt_stream(
            generation_cancellation.clone(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: NEVER_COMPLETING_GRAMMAR.to_owned(),
                    root: "root".to_owned(),
                }),
                max_tokens: i32::MAX,
                raw_prompt: "Repeat the word apple.".to_owned(),
            },
        )
        .await?;

    cluster
        .wait_for_buffered_request_count(1, ObservationWindow::release())
        .await
        .context("the generation request must wait in the buffer while the agent has no model")?;

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                chat_template_override: Some(ChatTemplate {
                    content: "{% for message in messages %}{{ message.content }}{% endfor %}"
                        .to_owned(),
                }),
                use_chat_template_override: true,
                ..qwen3_desired_state_with_embeddings(true)
            },
        )
        .await?;

    generation_stream
        .next()
        .await
        .context("the buffered generation request must start streaming tokens")??;

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "test".to_owned(),
                    id: "doc1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await?;

    assert_eq!(
        collected.embedding_rejected_due_to_active_token_generation_count,
        1
    );
    assert!(collected.embeddings.is_empty());

    generation_cancellation.cancel();
    drop(generation_stream);

    cluster.shutdown().await?;

    Ok(())
}
