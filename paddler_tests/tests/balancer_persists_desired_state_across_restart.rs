use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_desired_state_across_restart() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");

    let ModelCard { reference } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let first_cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: Some(desired_state.clone()),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    first_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let second_cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: None,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let restored_state = second_cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read restored desired state");

    assert_eq!(restored_state.model, desired_state.model);

    second_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
