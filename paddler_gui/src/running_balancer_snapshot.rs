use anyhow::Context;
use anyhow::Result;
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
    ) -> Result<Self> {
        let mut agent_snapshots = agent_controller_pool
            .make_snapshot()
            .context("Failed to collect agent controller snapshots")?
            .agents;

        sort_agent_snapshots_by_label(&mut agent_snapshots);

        let balancer_applicable_state =
            balancer_applicable_state_holder.get_balancer_applicable_state();

        Ok(Self {
            agent_snapshots,
            balancer_applicable_state,
            balancer_desired_state,
        })
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use paddler_balancer::agent_controller_pool::AgentControllerPool;
    use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
    use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::inference_parameters::InferenceParameters;

    use super::*;

    fn make_applicable_state() -> BalancerApplicableState {
        BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::LocalToAgent("configured_model".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        }
    }

    #[test]
    fn empty_inputs_produce_empty_snapshot() -> Result<()> {
        let pool = AgentControllerPool::default();
        let holder = BalancerApplicableStateHolder::new(BalancerApplicableState::from(
            BalancerDesiredState::default(),
        ));

        let snapshot =
            RunningBalancerSnapshot::build(&pool, &holder, BalancerDesiredState::default())?;

        assert!(snapshot.agent_snapshots.is_empty());
        assert_eq!(
            snapshot.balancer_desired_state,
            BalancerDesiredState::default()
        );

        Ok(())
    }

    #[test]
    fn carries_applicable_and_desired_state() -> Result<()> {
        let pool = AgentControllerPool::default();
        let holder = BalancerApplicableStateHolder::new(BalancerApplicableState::from(
            BalancerDesiredState::default(),
        ));
        holder.set_balancer_applicable_state(make_applicable_state());

        let desired = BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("requested_model".to_owned()),
            ..BalancerDesiredState::default()
        };

        let snapshot = RunningBalancerSnapshot::build(&pool, &holder, desired.clone())?;

        let applicable = snapshot.balancer_applicable_state;

        assert_eq!(
            applicable.agent_desired_state.model,
            AgentDesiredModel::LocalToAgent("configured_model".to_owned())
        );
        assert_eq!(snapshot.balancer_desired_state.model, desired.model);

        Ok(())
    }
}
