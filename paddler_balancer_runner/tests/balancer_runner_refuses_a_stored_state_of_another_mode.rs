use tempfile::TempDir;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use paddler_balancer_runner::balancer_runner::BalancerRunner;
use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_state_database::file::File as StateDatabaseFile;
use paddler_state_database::state_database::StateDatabase as _;
use paddler_state_database::state_database_error::StateDatabaseError;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_refuses_a_stored_state_of_another_mode() {
    let state_database_directory =
        TempDir::new().expect("a temporary state database directory must be creatable");
    let state_database_path = state_database_directory.path().join("state.json");
    let (balancer_desired_state_tx, _balancer_desired_state_rx) = watch::channel(
        BalancerDesiredState::unconfigured(InferenceMode::Embeddings),
    );

    StateDatabaseFile::new(
        balancer_desired_state_tx,
        InferenceMode::Embeddings,
        state_database_path.clone(),
    )
    .read_balancer_desired_state()
    .await
    .expect("an embeddings state database must be created");

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.serving_mode = BalancerServingMode::TextGeneration {
        openai_service_configuration: None,
    };
    params.runner_config.state_database_type = StateDatabaseType::File(state_database_path);

    let start_error = BalancerRunner::start(params)
        .await
        .err()
        .expect("a text generation runner must not start from an embeddings state database");

    assert!(matches!(
        start_error,
        BalancerRunnerError::StateDatabaseReadFailed {
            source: StateDatabaseError::StoredStateServesAnotherMode {
                cluster_inference_mode,
                stored_inference_mode,
            }
        } if cluster_inference_mode == InferenceMode::TextGeneration
            && stored_inference_mode == InferenceMode::Embeddings
    ));
}
