use std::collections::BTreeSet;
use std::time::Duration;

use futures_util::StreamExt;
use integration_tests::AGENT_DESIRED_MODEL;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

struct EmbeddingsTestCluster {
    balancer: ManagedBalancer,
    _agent: ManagedAgent,
    _state_db: NamedTempFile,
}

async fn spawn_embeddings_cluster(
    inference_parameters: InferenceParameters,
) -> EmbeddingsTestCluster {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters,
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("embeddings-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    EmbeddingsTestCluster {
        balancer,
        _agent: agent,
        _state_db: state_db,
    }
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
    let cluster = spawn_embeddings_cluster(InferenceParameters::default()).await;

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
