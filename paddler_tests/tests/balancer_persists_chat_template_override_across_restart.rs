use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_chat_template_override_across_restart() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");

    let ModelCard { reference } = qwen3_0_6b();

    let desired_state = desired_state_with_chat_template_override(
        BalancerDesiredState {
            model: AgentDesiredModel::HuggingFace(reference),
            ..BalancerDesiredState::default()
        },
        ChatTemplate {
            content: "{{ messages | tojson }}".to_owned(),
        },
    );

    let first_cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state.clone())),
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
        desired_state: ClusterDesiredState::KeepStored,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let restored_state = second_cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read restored desired state");

    assert_eq!(restored_state, desired_state);

    second_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
