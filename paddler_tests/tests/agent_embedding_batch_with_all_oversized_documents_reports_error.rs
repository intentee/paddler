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

const N_BATCH: u32 = 64;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embedding_batch_with_all_oversized_documents_reports_error() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        model_runtime_parameters: ModelRuntimeParameters {
            n_batch: BatchSize::try_from(N_BATCH).expect("the value must fit its target type"),
            context_size: NonZeroU32::try_from(2048).expect("the value must fit its target type"),
            ..ModelRuntimeParameters::default()
        },
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let huge_content = "The quick brown fox jumps over the lazy dog. ".repeat(40);

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![
                    EmbeddingInputDocument {
                        content: huge_content.clone(),
                        id: "huge-1".to_owned(),
                    },
                    EmbeddingInputDocument {
                        content: huge_content,
                        id: "huge-2".to_owned(),
                    },
                ],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(
        collected.embeddings.len(),
        0,
        "no embeddings should be produced when all documents are oversized",
    );
    assert_eq!(
        collected.oversized_documents.len(),
        2,
        "both oversized documents should be reported",
    );
    assert!(
        collected.saw_done,
        "stream must terminate with the balancer's final Done so the client unblocks",
    );
    assert_eq!(
        collected.no_embeddings_produced_count,
        1,
        "the agent must terminate its sub-stream with a single NoEmbeddingsProduced variant when zero embeddings are produced; got oversized_documents: {:?}, errors: {:?}",
        collected
            .oversized_documents
            .iter()
            .map(|details| &details.source_document_id)
            .collect::<Vec<_>>(),
        collected.failures,
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
