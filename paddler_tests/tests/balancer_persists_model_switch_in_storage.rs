use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_model_switch_in_storage() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");

    let ModelCard { reference } = qwen3_0_6b();

    let initial_state = BalancerDesiredState {
        model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
        ..BalancerDesiredState::default()
    };

    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: ClusterDesiredState::Apply(Box::new(initial_state.clone())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let observed_initial = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read initial desired state");

    assert_eq!(observed_initial.model, initial_state.model);

    let switched_state = BalancerDesiredState {
        model: ModelSource::LocalToAgent("/tmp/alternative-model.gguf".to_owned())
            .into_agent_desired_model(),
        ..BalancerDesiredState::default()
    };

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &switched_state)
        .await
        .expect("failed to switch desired model");

    let observed_switched = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read switched desired state");

    assert_eq!(observed_switched.model, switched_state.model);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
