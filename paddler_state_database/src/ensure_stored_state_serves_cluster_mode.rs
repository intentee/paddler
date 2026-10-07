use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

use crate::state_database_error::StateDatabaseError;

pub fn ensure_stored_state_serves_cluster_mode(
    cluster_inference_mode: InferenceMode,
    stored_desired_state: BalancerDesiredState,
) -> Result<BalancerDesiredState, StateDatabaseError> {
    let stored_inference_mode = stored_desired_state.inference_settings.inference_mode();

    if stored_inference_mode == cluster_inference_mode {
        Ok(stored_desired_state)
    } else {
        Err(StateDatabaseError::StoredStateServesAnotherMode {
            cluster_inference_mode,
            stored_inference_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::ensure_stored_state_serves_cluster_mode;
    use crate::state_database_error::StateDatabaseError;

    #[test]
    fn a_stored_state_of_the_cluster_mode_is_returned() {
        let stored_desired_state = BalancerDesiredState::unconfigured(InferenceMode::Embeddings);

        assert_eq!(
            ensure_stored_state_serves_cluster_mode(
                InferenceMode::Embeddings,
                stored_desired_state.clone()
            )
            .unwrap(),
            stored_desired_state
        );
    }

    #[test]
    fn a_stored_state_of_another_mode_is_refused() {
        assert!(matches!(
            ensure_stored_state_serves_cluster_mode(
                InferenceMode::TextGeneration,
                BalancerDesiredState::unconfigured(InferenceMode::Embeddings)
            ),
            Err(StateDatabaseError::StoredStateServesAnotherMode {
                cluster_inference_mode: InferenceMode::TextGeneration,
                stored_inference_mode: InferenceMode::Embeddings,
            })
        ));
    }
}
