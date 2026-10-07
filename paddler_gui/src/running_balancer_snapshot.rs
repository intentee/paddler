use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

use crate::sort_agent_snapshots_by_label::sort_agent_snapshots_by_label;

#[derive(Clone, Debug)]
pub struct RunningBalancerSnapshot {
    pub agent_snapshots: Vec<AgentControllerSnapshot>,
    pub balancer_applicable_state: BalancerApplicableState,
    pub balancer_desired_state: BalancerDesiredState,
}

impl RunningBalancerSnapshot {
    pub fn build(
        agent_controller_pool: &AgentControllerPool,
        balancer_applicable_state_holder: &BalancerApplicableStateHolder,
        balancer_desired_state: BalancerDesiredState,
    ) -> Self {
        let mut agent_snapshots = agent_controller_pool.make_snapshot().agents;

        sort_agent_snapshots_by_label(&mut agent_snapshots);

        let balancer_applicable_state =
            balancer_applicable_state_holder.get_balancer_applicable_state();

        Self {
            agent_snapshots,
            balancer_applicable_state,
            balancer_desired_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_balancer::agent_controller_pool::AgentControllerPool;
    use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
    use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::RunningBalancerSnapshot;

    #[test]
    fn carries_the_applicable_state_apart_from_the_desired_state() {
        let applied_model = AgentDesiredModel::LocalToAgent("configured_model".to_owned());
        let requested_model = AgentDesiredModel::LocalToAgent("requested_model".to_owned());

        let snapshot = RunningBalancerSnapshot::build(
            &AgentControllerPool::new(InferenceMode::TextGeneration),
            &BalancerApplicableStateHolder::new(BalancerApplicableState::from(
                BalancerDesiredState {
                    model: applied_model.clone(),
                    ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
                },
            )),
            BalancerDesiredState {
                model: requested_model.clone(),
                ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
            },
        );

        assert_eq!(
            snapshot.balancer_applicable_state.agent_desired_state.model,
            applied_model
        );
        assert_eq!(snapshot.balancer_desired_state.model, requested_model);
    }
}
