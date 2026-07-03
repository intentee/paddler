use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClusterPromptingMode {
    Enabled,
    DisabledForEmbeddings,
}

impl ClusterPromptingMode {
    #[must_use]
    pub fn from_applicable_state_holder(
        balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    ) -> Self {
        if let Some(agent_desired_state) =
            balancer_applicable_state_holder.get_agent_desired_state()
            && agent_desired_state.inference_parameters.enable_embeddings
        {
            Self::DisabledForEmbeddings
        } else {
            Self::Enabled
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::inference_parameters::InferenceParameters;

    use super::ClusterPromptingMode;
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
    fn enabled_when_state_is_not_set() {
        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();

        assert_eq!(
            ClusterPromptingMode::from_applicable_state_holder(&balancer_applicable_state_holder),
            ClusterPromptingMode::Enabled
        );
    }

    #[test]
    fn enabled_when_embeddings_are_disabled() {
        assert_eq!(
            ClusterPromptingMode::from_applicable_state_holder(&holder_with_embeddings(false)),
            ClusterPromptingMode::Enabled
        );
    }

    #[test]
    fn disabled_for_embeddings_when_embeddings_are_enabled() {
        assert_eq!(
            ClusterPromptingMode::from_applicable_state_holder(&holder_with_embeddings(true)),
            ClusterPromptingMode::DisabledForEmbeddings
        );
    }
}
