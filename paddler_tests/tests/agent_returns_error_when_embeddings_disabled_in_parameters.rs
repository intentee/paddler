#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_error_when_embeddings_disabled_in_parameters() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let cluster = InProcessCluster::start(InProcessClusterParams {
        spawn_agent: true,
        slots_per_agent: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let outcome = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello world".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await;

    if let Ok(stream) = outcome {
        let collected = collect_embedding_results(stream).await?;

        assert!(
            collected.embeddings.is_empty(),
            "no embeddings should be returned when embeddings are disabled"
        );
        assert!(
            !collected.errors.is_empty(),
            "stream must report at least one embedding error when embeddings are disabled"
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
