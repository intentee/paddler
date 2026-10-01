#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const N_BATCH: usize = 64;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_oversized_document_and_embeds_the_rest() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        inference_parameters: InferenceParameters {
            n_batch: BatchSize::try_from(
                u32::try_from(N_BATCH).expect("the value must fit its target type"),
            )
            .expect("the value must fit its target type"),
            context_size: NonZeroU32::try_from(2048).expect("the value must fit its target type"),
            enable_embeddings: true,
            ..InferenceParameters::deterministic()
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
                        content: "ok".to_owned(),
                        id: "tiny".to_owned(),
                    },
                    EmbeddingInputDocument {
                        content: huge_content,
                        id: "huge".to_owned(),
                    },
                ],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert!(
        collected.saw_done,
        "stream must terminate with Done even when one document is oversized",
    );
    assert!(
        collected.errors.is_empty(),
        "no generic EmbeddingResult::Error events should be emitted; got {:?}",
        collected.errors,
    );

    assert_eq!(
        collected.oversized_documents.len(),
        1,
        "exactly one DocumentExceedsBatchSize event expected; got {:?}",
        collected
            .oversized_documents
            .iter()
            .map(|details| &details.source_document_id)
            .collect::<Vec<_>>(),
    );

    let oversized = &collected.oversized_documents[0];

    assert_eq!(oversized.source_document_id, "huge");
    assert_eq!(oversized.n_batch, N_BATCH);
    assert!(
        oversized.document_tokens > oversized.n_batch,
        "document_tokens ({}) must exceed n_batch ({}) for the assertion to be meaningful",
        oversized.document_tokens,
        oversized.n_batch,
    );

    assert_eq!(
        collected.embeddings.len(),
        1,
        "the small document must still be embedded; got {:?}",
        collected
            .embeddings
            .iter()
            .map(|produced| &produced.embedding.source_document_id)
            .collect::<Vec<_>>(),
    );
    assert_eq!(collected.embeddings[0].embedding.source_document_id, "tiny",);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
