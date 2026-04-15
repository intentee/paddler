#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::collections::BTreeSet;

use futures_util::StreamExt;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use serial_test::file_serial;

fn embedding_model() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "Qwen3-Embedding-0.6B-Q8_0.gguf".to_owned(),
        repo_id: "Qwen/Qwen3-Embedding-0.6B-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
}

async fn spawn_embeddings_cluster(inference_parameters: InferenceParameters) -> ManagedCluster {
    ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "embeddings-agent".to_string(),
        desired_state: paddler_types::balancer_desired_state::BalancerDesiredState {
            inference_parameters,
            model: embedding_model(),
            ..ManagedClusterParams::default().desired_state
        },
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster")
}

async fn spawn_non_embedding_cluster() -> ManagedCluster {
    ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "non-embeddings-agent".to_string(),
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster")
}

async fn collect_embeddings_from_stream(
    stream: &mut (
             impl StreamExt<
        Item = Result<paddler_types::inference_client::Message, paddler_client::Error>,
    > + Unpin
         ),
) -> Vec<Embedding> {
    let mut embeddings = Vec::new();

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            paddler_types::inference_client::Message::Response(envelope) => {
                match envelope.response {
                    Response::Embedding(EmbeddingResult::Embedding(embedding)) => {
                        embeddings.push(embedding);
                    }
                    Response::Embedding(EmbeddingResult::Done) => {}
                    Response::Embedding(EmbeddingResult::Error(description)) => {
                        panic!("unexpected embedding error: {description}");
                    }
                    other => {
                        panic!("unexpected response variant: {other:?}");
                    }
                }
            }
            paddler_types::inference_client::Message::Error(envelope) => {
                panic!(
                    "unexpected error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    embeddings
}

fn make_embedding_params(
    documents: Vec<(&str, &str)>,
    normalization_method: EmbeddingNormalizationMethod,
) -> GenerateEmbeddingBatchParams {
    GenerateEmbeddingBatchParams {
        input_batch: documents
            .into_iter()
            .map(|(id, content)| EmbeddingInputDocument {
                content: content.to_string(),
                id: id.to_string(),
            })
            .collect(),
        normalization_method,
    }
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_fail_when_disabled() {
    let cluster = spawn_non_embedding_cluster().await;

    let params = make_embedding_params(
        vec![("doc-1", "Hello world")],
        EmbeddingNormalizationMethod::None,
    );

    let result = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await;

    assert!(
        result.is_err(),
        "embedding request should fail when embeddings are disabled"
    );
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_succeed_with_matching_document_ids() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![
            ("doc-alpha", "The quick brown fox jumps over the lazy dog"),
            (
                "doc-beta",
                "Machine learning is a subset of artificial intelligence",
            ),
        ],
        EmbeddingNormalizationMethod::None,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 2, "should receive exactly 2 embeddings");

    let returned_ids: BTreeSet<String> = embeddings
        .iter()
        .map(|embedding| embedding.source_document_id.clone())
        .collect();

    let expected_ids: BTreeSet<String> =
        BTreeSet::from(["doc-alpha".to_string(), "doc-beta".to_string()]);

    assert_eq!(returned_ids, expected_ids);
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_with_l2_normalization() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![("doc-l2", "Testing L2 normalization on embeddings")],
        EmbeddingNormalizationMethod::L2,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 1, "should receive exactly 1 embedding");

    let embedding = &embeddings[0];

    assert!(
        matches!(
            embedding.normalization_method,
            EmbeddingNormalizationMethod::L2
        ),
        "normalization method should be L2"
    );

    let l2_norm: f32 = embedding
        .embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();

    assert!(
        (l2_norm - 1.0).abs() < 1e-4,
        "L2 norm should be approximately 1.0, got {l2_norm}"
    );
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_with_rms_norm_normalization() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![("doc-rms", "Testing RMS normalization on embeddings")],
        EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 },
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 1, "should receive exactly 1 embedding");

    assert!(
        matches!(
            embeddings[0].normalization_method,
            EmbeddingNormalizationMethod::RmsNorm { .. }
        ),
        "normalization method should be RmsNorm"
    );
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_with_no_normalization() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![("doc-none", "Testing no normalization on embeddings")],
        EmbeddingNormalizationMethod::None,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 1, "should receive exactly 1 embedding");

    assert!(
        matches!(
            embeddings[0].normalization_method,
            EmbeddingNormalizationMethod::None
        ),
        "normalization method should be None"
    );
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_context_size_does_not_affect_batch_distribution() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        batch_n_tokens: 64,
        context_size: 512,
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![
            (
                "doc-chunk-1",
                "This is the first document with enough content to contribute meaningfully to the batch size calculation",
            ),
            (
                "doc-chunk-2",
                "This is the second document that should be processed in a potentially different batch from the first",
            ),
            (
                "doc-chunk-3",
                "This is the third document adding more content to ensure the total exceeds the configured chunk limit",
            ),
            (
                "doc-chunk-4",
                "This is the fourth document which should demonstrate that batching distributes across agent requests",
            ),
        ],
        EmbeddingNormalizationMethod::None,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(
        embeddings.len(),
        4,
        "should receive all 4 embeddings despite batch distribution"
    );

    let returned_ids: BTreeSet<String> = embeddings
        .iter()
        .map(|embedding| embedding.source_document_id.clone())
        .collect();

    let expected_ids: BTreeSet<String> = BTreeSet::from([
        "doc-chunk-1".to_string(),
        "doc-chunk-2".to_string(),
        "doc-chunk-3".to_string(),
        "doc-chunk-4".to_string(),
    ]);

    assert_eq!(returned_ids, expected_ids);
}

#[tokio::test]
#[file_serial]
async fn test_embeddings_have_same_dimensions() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let params = make_embedding_params(
        vec![
            ("doc-short", "Hello"),
            (
                "doc-medium",
                "The quick brown fox jumped over the lazy dog.",
            ),
            (
                "doc-long",
                "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection.",
            ),
        ],
        EmbeddingNormalizationMethod::None,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 3, "should receive exactly 3 embeddings");

    let first_dimension = embeddings[0].embedding.len();

    assert!(first_dimension > 0, "embedding dimension must be positive");

    for embedding in &embeddings {
        assert_eq!(
            embedding.embedding.len(),
            first_dimension,
            "all embeddings must have the same dimension, but {} has {} instead of {}",
            embedding.source_document_id,
            embedding.embedding.len(),
            first_dimension
        );
    }
}

#[tokio::test]
#[file_serial]
async fn test_identical_documents_produce_identical_embeddings() {
    let cluster = spawn_embeddings_cluster(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })
    .await;

    let repeated_content = "Deterministic embedding output test.";

    let params = make_embedding_params(
        vec![
            ("doc-first", repeated_content),
            ("doc-second", repeated_content),
        ],
        EmbeddingNormalizationMethod::None,
    );

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .generate_embedding_batch(&params)
        .await
        .expect("embedding request should succeed");

    let embeddings = collect_embeddings_from_stream(&mut stream).await;

    assert_eq!(embeddings.len(), 2, "should receive exactly 2 embeddings");

    let first = embeddings
        .iter()
        .find(|embedding| embedding.source_document_id == "doc-first")
        .expect("first embedding missing");

    let second = embeddings
        .iter()
        .find(|embedding| embedding.source_document_id == "doc-second")
        .expect("second embedding missing");

    assert_eq!(
        first.embedding, second.embedding,
        "identical documents must produce identical embedding vectors"
    );
}
