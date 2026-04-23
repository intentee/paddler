#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::io::Write;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use serial_test::file_serial;
use tempfile::NamedTempFile;

fn issue_cluster_params(desired_state: BalancerDesiredState) -> ManagedClusterParams {
    ManagedClusterParams {
        agent_name: "issue-test-agent".to_owned(),
        agent_slots: 1,
        desired_state,
        wait_for_slots: false,
        ..ManagedClusterParams::default()
    }
}

#[tokio::test]
#[file_serial]
async fn test_model_file_does_not_exist() -> Result<()> {
    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    let issue = cluster
        .balancer
        .wait_for_agent_issue(|issue| matches!(issue, AgentIssue::ModelFileDoesNotExist(_)))
        .await?;

    match issue {
        AgentIssue::ModelFileDoesNotExist(model_path) => {
            assert_eq!(model_path.model_path, "/nonexistent/model.gguf");
        }
        other => return Err(anyhow!("expected ModelFileDoesNotExist, got {other:?}")),
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_model_cannot_be_loaded() -> Result<()> {
    let mut corrupt_model = NamedTempFile::new().context("failed to create temp file")?;
    corrupt_model
        .write_all(b"this is not a valid gguf model file")
        .context("failed to write corrupt model")?;

    let corrupt_model_path = corrupt_model
        .path()
        .to_str()
        .context("temp file path is not valid UTF-8")?
        .to_owned();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent(corrupt_model_path.clone()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    let issue = cluster
        .balancer
        .wait_for_agent_issue(|issue| matches!(issue, AgentIssue::ModelCannotBeLoaded(_)))
        .await?;

    match issue {
        AgentIssue::ModelCannotBeLoaded(model_path) => {
            assert_eq!(model_path.model_path, corrupt_model_path);
        }
        other => return Err(anyhow!("expected ModelCannotBeLoaded, got {other:?}")),
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_huggingface_model_does_not_exist() -> Result<()> {
    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "nonexistent.gguf".to_owned(),
            repo_id: "nonexistent-org/nonexistent-model-gguf".to_owned(),
            revision: "main".to_owned(),
        }),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    cluster
        .balancer
        .wait_for_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::HuggingFaceModelDoesNotExist(_) | AgentIssue::HuggingFacePermissions(_)
            )
        })
        .await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_unable_to_find_chat_template() -> Result<()> {
    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "nomic-embed-text-v1.5.Q2_K.gguf".to_owned(),
            repo_id: "nomic-ai/nomic-embed-text-v1.5-GGUF".to_owned(),
            revision: "main".to_owned(),
        }),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    cluster
        .balancer
        .wait_for_agent_issue(|issue| matches!(issue, AgentIssue::UnableToFindChatTemplate(_)))
        .await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_chat_template_does_not_compile() -> Result<()> {
    let desired_state = BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: "{{invalid jinja template".to_owned(),
        }),
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    cluster
        .balancer
        .wait_for_agent_issue(|issue| matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_)))
        .await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_multimodal_projection_cannot_be_loaded() -> Result<()> {
    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::LocalToAgent(
            "/nonexistent/projection.bin".to_owned(),
        ),
        use_chat_template_override: false,
    };

    let cluster = ManagedCluster::spawn(issue_cluster_params(desired_state))
        .await
        .context("failed to spawn cluster")?;

    cluster
        .balancer
        .wait_for_agent_issue(|issue| {
            matches!(issue, AgentIssue::MultimodalProjectionCannotBeLoaded(_))
        })
        .await?;

    Ok(())
}
