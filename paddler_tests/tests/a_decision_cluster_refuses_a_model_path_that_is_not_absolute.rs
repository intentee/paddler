use std::path::PathBuf;

use paddler_model_source::model_source_error::ModelSourceError;
use paddler_tests::decision_backbone::DecisionBackbone;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_desired_state::DecisionDesiredState;

const RELATIVE_MODEL_PATH: &str = "model.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_cluster_refuses_a_model_path_that_is_not_absolute() {
    assert_eq!(
        DecisionClusterParams {
            desired_state: DecisionDesiredState {
                backbone: DecisionBackbone::LocalFile(PathBuf::from(RELATIVE_MODEL_PATH)),
                ..DecisionDesiredState::default()
            },
            ..DecisionClusterParams::default()
        }
        .start_cluster()
        .await
        .err()
        .map(|startup_error| startup_error.to_string()),
        Some(
            ModelSourceError::LocalModelPathNotAbsolute {
                path: PathBuf::from(RELATIVE_MODEL_PATH),
            }
            .to_string()
        )
    );
}
