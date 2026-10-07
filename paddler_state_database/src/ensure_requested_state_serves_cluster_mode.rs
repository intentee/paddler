use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

use crate::state_database_error::StateDatabaseError;

pub fn ensure_requested_state_serves_cluster_mode(
    cluster_inference_mode: InferenceMode,
    requested_desired_state: &BalancerDesiredState,
) -> Result<(), StateDatabaseError> {
    let requested_inference_mode = requested_desired_state.inference_settings.inference_mode();

    if requested_inference_mode == cluster_inference_mode {
        Ok(())
    } else {
        Err(StateDatabaseError::RequestedStateServesAnotherMode {
            cluster_inference_mode,
            requested_inference_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::ensure_requested_state_serves_cluster_mode;
    use crate::state_database_error::StateDatabaseError;

    #[test]
    fn a_requested_state_of_the_cluster_mode_is_accepted() {
        assert!(
            ensure_requested_state_serves_cluster_mode(
                InferenceMode::TextGeneration,
                &BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
            )
            .is_ok()
        );
    }

    #[test]
    fn a_requested_state_of_another_mode_is_refused() {
        assert!(matches!(
            ensure_requested_state_serves_cluster_mode(
                InferenceMode::Embeddings,
                &BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
            ),
            Err(StateDatabaseError::RequestedStateServesAnotherMode {
                cluster_inference_mode: InferenceMode::Embeddings,
                requested_inference_mode: InferenceMode::TextGeneration,
            })
        ));
    }
}
