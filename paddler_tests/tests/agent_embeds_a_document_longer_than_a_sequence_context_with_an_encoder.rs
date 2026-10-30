#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const CONTEXT_SIZE: u32 = 512;
const SLOTS: u16 = 4;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embeds_a_document_longer_than_a_sequence_context_with_an_encoder() {
    let context_size =
        NonZeroU32::new(CONTEXT_SIZE).expect("the context size must be greater than zero");
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: AgentConfig::uniform(1, SLOTS),
        model_runtime_parameters: ModelRuntimeParameters {
            context_size,
            n_batch: BatchSize::default().at_most(context_size),
            ..ModelRuntimeParameters::default()
        },
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "The quick brown fox jumps over the lazy dog. ".repeat(30),
                    id: "longer-than-a-sequence".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(collected.failures, vec![]);
    assert_eq!(
        collected
            .embeddings
            .iter()
            .map(|produced| produced.embedding.source_document_id.as_str())
            .collect::<Vec<_>>(),
        vec!["longer-than-a-sequence"]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
