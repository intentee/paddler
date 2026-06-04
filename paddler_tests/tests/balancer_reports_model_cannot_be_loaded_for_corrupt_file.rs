#![cfg(feature = "tests_that_use_llms")]

use std::io::Write as _;

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tempfile::NamedTempFile;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_cannot_be_loaded_for_corrupt_file() -> Result<()> {
    let mut corrupt_model = NamedTempFile::new()?;

    corrupt_model.write_all(b"this is not a valid gguf model file")?;

    let corrupt_model_path = corrupt_model
        .path()
        .to_str()
        .context("temp file path is not valid UTF-8")?
        .to_owned();

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent(corrupt_model_path.clone()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let expected_path = corrupt_model_path.clone();

    cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent.issues.iter().any(|issue| {
                        matches!(issue, AgentIssue::ModelCannotBeLoaded(model_path)
                            if model_path.model_path == expected_path)
                    })
            })
        })
        .await
        .context("balancer should report ModelCannotBeLoaded for corrupt GGUF")?;

    cluster.shutdown().await?;

    Ok(())
}
