use actix_web::Error;
use actix_web::error::ErrorNotImplemented;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub fn require_token_generation_enabled(
    balancer_applicable_state_holder: &BalancerApplicableStateHolder,
) -> Result<(), Error> {
    if let Some(agent_desired_state) = balancer_applicable_state_holder.get_agent_desired_state()
        && agent_desired_state.inference_parameters.enable_embeddings
    {
        return Err(ErrorNotImplemented(
            "Token generation is disabled while the cluster is configured for embeddings",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::inference_parameters::InferenceParameters;

    use super::require_token_generation_enabled;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

    fn holder_with_embeddings(enable_embeddings: bool) -> BalancerApplicableStateHolder {
        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();

        balancer_applicable_state_holder.set_balancer_applicable_state(Some(
            BalancerApplicableState {
                agent_desired_state: AgentDesiredState {
                    chat_template_override: None,
                    inference_parameters: InferenceParameters {
                        enable_embeddings,
                        ..InferenceParameters::default()
                    },
                    model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                    multimodal_projection: AgentDesiredModel::None,
                },
            },
        ));

        balancer_applicable_state_holder
    }

    #[test]
    fn allows_generation_when_state_is_not_set() {
        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();

        assert!(require_token_generation_enabled(&balancer_applicable_state_holder).is_ok());
    }

    #[test]
    fn allows_generation_when_embeddings_are_disabled() {
        assert!(require_token_generation_enabled(&holder_with_embeddings(false)).is_ok());
    }

    #[test]
    fn rejects_generation_when_embeddings_are_enabled() {
        assert!(require_token_generation_enabled(&holder_with_embeddings(true)).is_err());
    }
}
