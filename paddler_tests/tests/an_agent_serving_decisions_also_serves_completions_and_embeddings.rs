#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_desired_state::DecisionDesiredState;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_decisions_also_serves_completions_and_embeddings() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        desired_state: DecisionDesiredState {
            embedding_parameters: EmbeddingParameters {
                pooling_type: PoolingType::Last,
                ..EmbeddingParameters::default()
            },
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    })
    .await
    .expect("a decision cluster must start");

    let decided = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision must be accepted");
    let generated = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the raw prompt must be accepted");
    let embedded = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "the quick brown fox jumps over the lazy dog".to_owned(),
                    id: "doc-1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(
        discriminant(&decided.terminal_result),
        decision_done_discriminant()
    );
    assert_eq!(
        generated
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );
    assert_eq!(
        embedded
            .embeddings
            .iter()
            .map(|embedding_with_producer| embedding_with_producer
                .embedding
                .source_document_id
                .clone())
            .collect::<Vec<_>>(),
        vec!["doc-1".to_owned()]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
