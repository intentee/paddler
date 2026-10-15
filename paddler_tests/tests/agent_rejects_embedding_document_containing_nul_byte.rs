#![cfg(feature = "tests_that_use_llms")]

use std::ffi::CString;

use llama_cpp_bindings::error::StringToTokenError;
use tokio_util::sync::CancellationToken;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const DOCUMENT_CONTENT_WITH_NUL_BYTE: &str = "before\0after";
const DOCUMENT_ID: &str = "nul-document";

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_embedding_document_containing_nul_byte() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],

        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: DOCUMENT_CONTENT_WITH_NUL_BYTE.to_owned(),
                    id: DOCUMENT_ID.to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    let nul_error = CString::new(DOCUMENT_CONTENT_WITH_NUL_BYTE)
        .expect_err("the document content must contain a nul byte");

    assert!(collected.embeddings.is_empty());
    assert_eq!(
        collected.failures,
        vec![EmbeddingResult::InputTokenizationFailed(
            rejection_description(
                Some("test-agent"),
                &EmbeddingError::InputTokenizationFailed {
                    source_document_id: DOCUMENT_ID.to_owned(),
                    source: StringToTokenError::NulError(nul_error),
                }
            )
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
