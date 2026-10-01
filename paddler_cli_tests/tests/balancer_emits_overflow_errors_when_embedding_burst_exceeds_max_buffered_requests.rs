#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_embedding_cluster::start_subprocess_embedding_cluster;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_emits_overflow_errors_when_embedding_burst_exceeds_max_buffered_requests() {
    const TOTAL_DOCUMENTS: usize = 16;

    let cluster = start_subprocess_embedding_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        EmbeddingClusterParams {
            agents: AgentConfig::uniform(4, 1),
            inference_parameters: InferenceParameters {
                embedding_batch_size: NonZeroUsize::MIN,
                enable_embeddings: true,
                ..InferenceParameters::deterministic()
            },
            max_buffered_requests: 4,
            ..EmbeddingClusterParams::default()
        },
    )
    .await
    .expect("the cluster must start");

    let input_batch: Vec<EmbeddingInputDocument> = (0..TOTAL_DOCUMENTS)
        .map(|index| EmbeddingInputDocument {
            content: format!("Overflow probe document {index}."),
            id: format!("doc-{index}"),
        })
        .collect();

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch,
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    let overflow_errors: Vec<_> = collected
        .wire_errors
        .iter()
        .filter(|wire_error| wire_error.code == 503)
        .collect();

    assert!(
        !overflow_errors.is_empty(),
        "expected at least one HTTP 503 \"Buffered requests overflow\" envelope, but saw none; wire_errors = {:?}",
        collected.wire_errors,
    );

    for overflow in &overflow_errors {
        assert_eq!(overflow.description, "Buffered requests overflow");
    }

    assert!(
        collected.saw_done,
        "stream must terminate cleanly with Done even when some sub-batches overflow",
    );

    assert_eq!(
        collected.embeddings.len() + collected.wire_errors.len(),
        TOTAL_DOCUMENTS,
        "every sub-batch must be accounted for as either a successful embedding or a wire error (503 overflow or 504 timeout): {} embeddings + {} wire errors ({} of which are 503 overflow) ≠ {TOTAL_DOCUMENTS}",
        collected.embeddings.len(),
        collected.wire_errors.len(),
        overflow_errors.len(),
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
