#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_embeddings::normalization::normalize_embedding::normalize_embedding;
use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_vector::EmbeddingVector;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const RMS_NORMALIZATION: EmbeddingNormalizationMethod =
    EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 };

#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_rms_normalized_embeddings_when_requested() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],

        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let embedding_of = |normalization_method| {
        cluster.generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "Testing RMS normalization on embeddings".to_owned(),
                    id: "doc-rms".to_owned(),
                }],
                normalization_method,
            },
        )
    };

    let unnormalized_embedding = embedding_of(EmbeddingNormalizationMethod::None)
        .await
        .expect("the unnormalized embedding batch must be accepted")
        .embeddings
        .into_iter()
        .map(|produced| produced.embedding)
        .next()
        .expect("the document must be embedded without normalization");
    let rms_normalized = embedding_of(RMS_NORMALIZATION)
        .await
        .expect("the RMS-normalized embedding batch must be accepted");

    assert!(rms_normalized.saw_done);
    assert_eq!(
        rms_normalized
            .embeddings
            .into_iter()
            .map(|produced| produced.embedding)
            .collect::<Vec<_>>(),
        vec![Embedding {
            embedding: EmbeddingVector::from(
                normalize_embedding(
                    unnormalized_embedding.embedding.to_vec(),
                    &RMS_NORMALIZATION
                )
                .expect("an embedding of the served model must fit RMS normalization")
            ),
            normalization_method: RMS_NORMALIZATION,
            ..unnormalized_embedding
        }]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
