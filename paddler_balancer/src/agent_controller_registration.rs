use std::sync::Arc;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

pub enum AgentControllerRegistration {
    DuplicateAgentId,
    Registered(RegisteredAgentControllerGuard),
}

impl AgentControllerRegistration {
    pub fn register(
        agent_controller_pool: &Arc<AgentControllerPool>,
        agent_controller: Arc<AgentController>,
        balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    ) -> Self {
        if agent_controller_pool
            .try_insert_agent_controller(&agent_controller, balancer_applicable_state_holder)
        {
            Self::Registered(RegisteredAgentControllerGuard {
                agent_controller,
                agent_controller_pool: agent_controller_pool.clone(),
            })
        } else {
            Self::DuplicateAgentId
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::FutureExt as _;
    use serde_json::Value;
    use serde_json::to_value;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use paddler_balancer_desired_state_fixture::desired_state_with_model::desired_state_with_model;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentControllerRegistration;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_params::AgentControllerParams;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_desired_state_notification::agent_desired_state_notification;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

    fn registering_agent_controller() -> Arc<AgentController> {
        Arc::new(AgentController::new(AgentControllerParams {
            connection_close: CancellationToken::new(),
            id: "registering".to_owned(),
            name: None,
            reported_status: SlotAggregatedStatusSnapshot::default(),
        }))
    }

    fn desired_state_notification_value(desired_state: &BalancerDesiredState) -> Value {
        to_value(agent_desired_state_notification(AgentDesiredState::from(
            desired_state,
        )))
        .unwrap()
    }

    #[test]
    fn registering_an_agent_sends_it_the_current_desired_state_before_anything_else() {
        let current_desired_state = desired_state_with_model("agent:///current/model.gguf");
        let (_applicable_state_tx, applicable_state_rx) = watch::channel(Arc::new(
            BalancerApplicableState::initial(current_desired_state.clone()),
        ));
        let agent_controller = registering_agent_controller();
        let _registration = AgentControllerRegistration::register(
            &Arc::new(AgentControllerPool::default()),
            agent_controller.clone(),
            &BalancerApplicableStateHolder::new(applicable_state_rx),
        );

        assert_eq!(
            agent_controller
                .agent_outbox
                .next()
                .now_or_never()
                .map(|first_message| to_value(first_message).unwrap()),
            Some(desired_state_notification_value(&current_desired_state))
        );
    }
}
