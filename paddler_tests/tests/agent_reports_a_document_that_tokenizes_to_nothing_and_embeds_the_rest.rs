#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_document_that_tokenizes_to_nothing_and_embeds_the_rest() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        embedding_parameters: EmbeddingParameters {
            pooling_type: PoolingType::Last,
            ..EmbeddingParameters::default()
        },
        model_card: smolvlm2_256m(),
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![
                    EmbeddingInputDocument {
                        content: String::new(),
                        id: "empty".to_owned(),
                    },
                    EmbeddingInputDocument {
                        content: "ok".to_owned(),
                        id: "ok".to_owned(),
                    },
                ],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert!(collected.saw_done);
    assert_eq!(collected.failures, Vec::new());
    assert_eq!(
        collected.documents_tokenized_to_nothing,
        vec![EmbeddingDocumentTokenizedToNothingDetails {
            source_document_id: "empty".to_owned(),
        }]
    );
    assert_eq!(
        collected
            .embeddings
            .iter()
            .map(|embedding_with_producer| embedding_with_producer
                .embedding
                .source_document_id
                .as_str())
            .collect::<Vec<_>>(),
        vec!["ok"]
    );
}
