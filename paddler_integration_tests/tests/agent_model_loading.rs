#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;
use serial_test::file_serial;

fn invalid_gguf_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/invalid.gguf").to_string()
}

fn invalid_mmproj_path() -> String {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/invalid_mmproj.gguf"
    )
    .to_string()
}

#[tokio::test]
#[file_serial]
async fn test_invalid_gguf_returns_error() {
    let model_path = invalid_gguf_path();

    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "model-loading-agent".to_string(),
        agent_slots: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent(model_path.clone()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots: false,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let issue = cluster
        .balancer
        .wait_for_agent_issue(|issue| matches!(issue, AgentIssue::ModelCannotBeLoaded(_)))
        .await;

    match issue {
        AgentIssue::ModelCannotBeLoaded(reported_path) => {
            assert_eq!(reported_path.model_path, model_path);
        }
        other => panic!("expected ModelCannotBeLoaded, got {other:?}"),
    }
}

#[tokio::test]
#[file_serial]
async fn test_invalid_mmproj_returns_error() {
    let mmproj_path = invalid_mmproj_path();

    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "model-loading-agent".to_string(),
        agent_slots: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AGENT_DESIRED_MODEL.clone(),
            multimodal_projection: AgentDesiredModel::LocalToAgent(mmproj_path.clone()),
            use_chat_template_override: false,
        },
        wait_for_slots: false,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let issue = cluster
        .balancer
        .wait_for_agent_issue(|issue| {
            matches!(issue, AgentIssue::MultimodalProjectionCannotBeLoaded(_))
        })
        .await;

    match issue {
        AgentIssue::MultimodalProjectionCannotBeLoaded(reported_path) => {
            assert_eq!(reported_path.model_path, mmproj_path);
        }
        other => panic!("expected MultimodalProjectionCannotBeLoaded, got {other:?}"),
    }
}
