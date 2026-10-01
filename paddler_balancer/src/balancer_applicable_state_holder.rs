use parking_lot::RwLock;
use tokio::sync::watch;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;

pub struct BalancerApplicableStateHolder {
    balancer_applicable_state: RwLock<BalancerApplicableState>,
    update_tx: watch::Sender<()>,
}

impl BalancerApplicableStateHolder {
    #[must_use]
    pub fn new(balancer_applicable_state: BalancerApplicableState) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            balancer_applicable_state: RwLock::new(balancer_applicable_state),
            update_tx,
        }
    }

    pub fn get_agent_desired_state(&self) -> AgentDesiredState {
        self.balancer_applicable_state
            .read()
            .agent_desired_state
            .clone()
    }

    pub fn get_balancer_applicable_state(&self) -> BalancerApplicableState {
        self.balancer_applicable_state.read().clone()
    }

    #[must_use]
    pub fn token_generation_mode(&self) -> ClusterTokenGenerationMode {
        if self
            .balancer_applicable_state
            .read()
            .agent_desired_state
            .inference_parameters
            .enable_embeddings
        {
            ClusterTokenGenerationMode::DisabledForEmbeddings
        } else {
            ClusterTokenGenerationMode::Enabled
        }
    }

    pub fn set_balancer_applicable_state(
        &self,
        balancer_applicable_state: BalancerApplicableState,
    ) {
        *self.balancer_applicable_state.write() = balancer_applicable_state;

        self.update_tx.send_replace(());
    }
}

impl SubscribesToUpdates for BalancerApplicableStateHolder {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

    use super::BalancerApplicableStateHolder;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;

    fn holder_with_embeddings(enable_embeddings: bool) -> BalancerApplicableStateHolder {
        BalancerApplicableStateHolder::new(BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    enable_embeddings,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        })
    }

    #[test]
    fn generates_tokens_when_embeddings_are_disabled() {
        assert_eq!(
            holder_with_embeddings(false).token_generation_mode(),
            ClusterTokenGenerationMode::Enabled
        );
    }

    #[test]
    fn disables_token_generation_when_embeddings_are_enabled() {
        assert_eq!(
            holder_with_embeddings(true).token_generation_mode(),
            ClusterTokenGenerationMode::DisabledForEmbeddings
        );
    }

    #[test]
    fn replacing_the_state_notifies_subscribers_and_is_read_back() {
        let holder = BalancerApplicableStateHolder::new(BalancerApplicableState::from(
            BalancerDesiredState::default(),
        ));
        let update_rx = holder.subscribe_to_updates();

        holder.set_balancer_applicable_state(BalancerApplicableState::from(BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
            ..BalancerDesiredState::default()
        }));

        assert!(
            update_rx
                .has_changed()
                .expect("the holder must keep its update sender")
        );
        assert_eq!(
            holder.get_agent_desired_state().model,
            AgentDesiredModel::LocalToAgent("model.gguf".to_owned())
        );
    }
}
