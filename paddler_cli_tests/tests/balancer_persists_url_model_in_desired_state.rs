#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;
use paddler::url_model_reference::UrlModelReference;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_url_model_in_desired_state() -> Result<()> {
    let configured_url = "https://example.invalid/persisted-model.gguf".to_owned();

    let cluster = start_subprocess_cluster(SubprocessClusterParams {
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
        ..SubprocessClusterParams::default()
    })
    .await?;

    let retrieved = cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
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
