use tokio::sync::watch;

use crate::agent_applicable_state::AgentApplicableState;

#[derive(Default)]
pub struct AgentApplicableStateHolder {
    agent_applicable_state_tx: watch::Sender<AgentApplicableState>,
}

impl AgentApplicableStateHolder {
    #[must_use]
    pub fn get_agent_applicable_state(&self) -> AgentApplicableState {
        self.agent_applicable_state_tx.borrow().clone()
    }

    pub fn set_agent_applicable_state(&self, agent_applicable_state: AgentApplicableState) {
        self.agent_applicable_state_tx
            .send_replace(agent_applicable_state);
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<AgentApplicableState> {
        let mut agent_applicable_state_rx = self.agent_applicable_state_tx.subscribe();

        agent_applicable_state_rx.mark_changed();

        agent_applicable_state_rx
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
    use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

    use super::AgentApplicableStateHolder;
    use crate::agent_applicable_state::AgentApplicableState;

    #[test]
    fn a_new_subscriber_sees_the_state_applied_before_it_subscribed() {
        let holder = AgentApplicableStateHolder::default();

        let applied_state = AgentApplicableState::Embeddings {
            embedding_parameters: EmbeddingParameters::default(),
            model_path: PathBuf::from("model.gguf"),
            model_runtime_parameters: ModelRuntimeParameters::default(),
        };

        holder.set_agent_applicable_state(applied_state.clone());

        let agent_applicable_state_rx = holder.subscribe();

        assert!(agent_applicable_state_rx.has_changed().unwrap());
        assert_eq!(*agent_applicable_state_rx.borrow(), applied_state);
    }
}
