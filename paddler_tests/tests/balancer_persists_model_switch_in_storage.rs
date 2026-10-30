use std::path::Path;

use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::temporary_state_database_file::TemporaryStateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_model_switch_in_storage() {
    let database =
        TemporaryStateDatabaseFile::new().expect("the state database file must be created");

    let ModelCard { reference } = qwen3_0_6b();

    let initial_state = BalancerDesiredState {
        model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
        ..BalancerDesiredState::default()
    };

    let first_cluster = start_cluster(ClusterParams {
        state_database_url: database.url.clone(),
        desired_state: ClusterDesiredState::Apply(Box::new(initial_state)),
        ..ClusterParams::without_agents()
    })
    .await
    .expect("the cluster must start");

    let switched_state = BalancerDesiredState {
        model: agent_desired_model_at(Path::new("/tmp/alternative-model.gguf"))
            .expect("the alternative model path must be absolute"),
        ..BalancerDesiredState::default()
    };

    first_cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &switched_state)
        .await
        .expect("failed to switch desired model");
    first_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let second_cluster = start_cluster(ClusterParams {
        state_database_url: database.url.clone(),
        desired_state: ClusterDesiredState::KeepStored,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("the cluster must restart from the stored desired state");

    assert_eq!(
        second_cluster
            .client_management
            .get_balancer_desired_state(CancellationToken::new())
            .await
            .expect("failed to read the restored desired state")
            .model,
        switched_state.model
    );

    second_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
