#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_url_model_in_desired_state() -> Result<()> {
    let configured_url = "https://example.invalid/persisted-model.gguf".to_owned();

    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::Url(UrlModelReference {
                url: configured_url.clone(),
            }),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let retrieved = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read balancer desired state")?;

    assert_eq!(
        retrieved.model,
        AgentDesiredModel::Url(UrlModelReference {
            url: configured_url,
        })
    );

    cluster.shutdown().await?;

    Ok(())
}
