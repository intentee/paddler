use std::path::PathBuf;

use paddler_model_source::model_source_error::ModelSourceError;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_desired_state::DecisionDesiredState;

const RELATIVE_POINTER_HEAD_PATH: &str = "pointer_head.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_cluster_refuses_a_pointer_head_path_that_is_not_absolute() {
    assert_eq!(
        DecisionClusterParams {
            desired_state: DecisionDesiredState {
                pointer_head: PathBuf::from(RELATIVE_POINTER_HEAD_PATH),
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
                path: PathBuf::from(RELATIVE_POINTER_HEAD_PATH),
            }
            .to_string()
        )
    );
}
