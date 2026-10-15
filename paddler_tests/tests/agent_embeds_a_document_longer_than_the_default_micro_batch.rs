#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const NOMIC_TRAINING_CONTEXT_SIZE: u32 = 2048;
const SENTENCES_EXCEEDING_THE_DEFAULT_MICRO_BATCH: usize = 100;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embeds_a_document_longer_than_the_default_micro_batch() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        model_runtime_parameters: ModelRuntimeParameters {
            context_size: NonZeroU32::try_from(NOMIC_TRAINING_CONTEXT_SIZE)
                .expect("the value must fit its target type"),
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
                    content: "The quick brown fox jumps over the lazy dog. "
                        .repeat(SENTENCES_EXCEEDING_THE_DEFAULT_MICRO_BATCH),
                    id: "long-document".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(
        collected
            .embeddings
            .iter()
            .map(|produced| produced.embedding.source_document_id.as_str())
            .collect::<Vec<_>>(),
        vec!["long-document"]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
