use anyhow::Context;
use anyhow::Result;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer_applicable_state::BalancerApplicableState;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler::produces_snapshot::ProducesSnapshot as _;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug, Default)]
pub struct RunningClusterSnapshot {
    pub agent_snapshots: Vec<AgentControllerSnapshot>,
    pub balancer_applicable_state: Option<BalancerApplicableState>,
    pub balancer_desired_state: BalancerDesiredState,
}

impl RunningClusterSnapshot {
    pub fn build(
        agent_controller_pool: &AgentControllerPool,
        balancer_applicable_state_holder: &BalancerApplicableStateHolder,
        balancer_desired_state: BalancerDesiredState,
    ) -> Result<Self> {
        let mut agent_snapshots = agent_controller_pool
            .make_snapshot()
            .context("Failed to collect agent controller snapshots")?
            .agents;

        agent_snapshots.sort_by(|left_agent, right_agent| {
            let left_label = left_agent.name.as_deref().unwrap_or(&left_agent.id);
            let right_label = right_agent.name.as_deref().unwrap_or(&right_agent.id);

            left_label.cmp(right_label)
        });

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
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::RwLock;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicUsize;

    use anyhow::Result;
    use paddler::agent_desired_state::AgentDesiredState;
    use paddler::atomic_value::AtomicValue;
    use paddler::balancer::agent_controller::AgentController;
    use paddler::balancer::agent_controller_pool::AgentControllerPool;
    use paddler::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use paddler::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use paddler::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use paddler::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler::balancer_applicable_state::BalancerApplicableState;
    use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_types::inference_parameters::InferenceParameters;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;

    fn make_agent_controller(id: &str, name: Option<&str>) -> Arc<AgentController> {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

        Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(0),
            download_current: AtomicValue::<AtomicUsize>::new(0),
            download_filename: RwLock::new(None),
            download_total: AtomicValue::<AtomicUsize>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: id.to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: name.map(str::to_owned),
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(0),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        })
    }

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
        let holder = BalancerApplicableStateHolder::default();

        let snapshot =
            RunningClusterSnapshot::build(&pool, &holder, BalancerDesiredState::default())?;

        assert!(snapshot.agent_snapshots.is_empty());
        assert!(snapshot.balancer_applicable_state.is_none());
        assert_eq!(
            snapshot.balancer_desired_state,
            BalancerDesiredState::default()
        );

        Ok(())
    }

    #[test]
    fn carries_applicable_and_desired_state() -> Result<()> {
        let pool = AgentControllerPool::default();
        let holder = BalancerApplicableStateHolder::default();
        holder.set_balancer_applicable_state(Some(make_applicable_state()));

        let desired = BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("requested_model".to_owned()),
            ..BalancerDesiredState::default()
        };

        let snapshot = RunningClusterSnapshot::build(&pool, &holder, desired.clone())?;

        let applicable = snapshot
            .balancer_applicable_state
            .ok_or_else(|| anyhow::anyhow!("applicable state should be carried"))?;

        assert_eq!(
            applicable.agent_desired_state.model,
            AgentDesiredModel::LocalToAgent("configured_model".to_owned())
        );
        assert_eq!(snapshot.balancer_desired_state.model, desired.model);

        Ok(())
    }

    #[test]
    fn sorts_agents_by_name_then_id() -> Result<()> {
        let pool = AgentControllerPool::default();
        let holder = BalancerApplicableStateHolder::default();

        pool.register_agent_controller(
            "id_z".to_owned(),
            make_agent_controller("id_z", Some("alpha")),
        )?;
        pool.register_agent_controller(
            "id_a".to_owned(),
            make_agent_controller("id_a", Some("bravo")),
        )?;
        pool.register_agent_controller("id_m".to_owned(), make_agent_controller("id_m", None))?;

        let snapshot =
            RunningClusterSnapshot::build(&pool, &holder, BalancerDesiredState::default())?;

        let labels: Vec<String> = snapshot
            .agent_snapshots
            .iter()
            .map(|agent| agent.name.clone().unwrap_or_else(|| agent.id.clone()))
            .collect();

        assert_eq!(labels, vec!["alpha", "bravo", "id_m"]);

        Ok(())
    }
}
