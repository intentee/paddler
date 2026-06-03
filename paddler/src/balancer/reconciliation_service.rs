use std::sync::Arc;

use crate::balancer_desired_state::BalancerDesiredState;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::converts_to_applicable_state::ConvertsToApplicableState as _;
use crate::sets_desired_state::SetsDesiredState as _;

async fn convert_to_applicable_state(
    balancer_desired_state: &BalancerDesiredState,
    agent_controller_pool: &AgentControllerPool,
    balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) -> Result<()> {
    let balancer_applicable_state = balancer_desired_state.to_applicable_state(()).await?;

    agent_controller_pool
        .set_desired_state(balancer_applicable_state.agent_desired_state.clone())
        .await?;
    balancer_applicable_state_holder.set_balancer_applicable_state(Some(balancer_applicable_state));

    *is_converted_to_applicable_state = true;

    Ok(())
}

async fn try_convert_to_applicable_state(
    balancer_desired_state: &BalancerDesiredState,
    agent_controller_pool: &AgentControllerPool,
    balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) {
    if let Err(err) = convert_to_applicable_state(
        balancer_desired_state,
        agent_controller_pool,
        balancer_applicable_state_holder,
        is_converted_to_applicable_state,
    )
    .await
    {
        error!("Failed to convert to applicable state: {err}");
    }
}

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state: BalancerDesiredState,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub is_converted_to_applicable_state: bool,
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "balancer::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
            agent_controller_pool,
            balancer_applicable_state_holder,
            mut balancer_desired_state,
            mut balancer_desired_state_rx,
            mut is_converted_to_applicable_state,
        } = *self;

        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if !is_converted_to_applicable_state {
                        try_convert_to_applicable_state(
                            &balancer_desired_state,
                            &agent_controller_pool,
                            &balancer_applicable_state_holder,
                            &mut is_converted_to_applicable_state,
                        ).await;
                    }
                },
                received_balancer_desired_state = balancer_desired_state_rx.recv() => {
                    is_converted_to_applicable_state = false;
                    balancer_desired_state = received_balancer_desired_state?;
                    try_convert_to_applicable_state(
                        &balancer_desired_state,
                        &agent_controller_pool,
                        &balancer_applicable_state_holder,
                        &mut is_converted_to_applicable_state,
                    ).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::convert_to_applicable_state;
    use super::try_convert_to_applicable_state;
    use crate::agent_state_application_status::AgentStateApplicationStatus;
    use crate::atomic_value::AtomicValue;
    use crate::balancer::agent_controller::AgentController;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::balancer_desired_state::BalancerDesiredState;

    fn agent_controller_with_dropped_receiver() -> Arc<AgentController> {
        let (agent_message_tx, agent_message_rx) = mpsc::unbounded_channel();

        drop(agent_message_rx);

        Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(0),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: "agent-test".to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(0),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        })
    }

    #[tokio::test]
    async fn convert_to_applicable_state_sets_flag_and_stores_state_for_empty_pool() {
        let balancer_desired_state = BalancerDesiredState::default();
        let agent_controller_pool = AgentControllerPool::default();
        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();
        let mut is_converted_to_applicable_state = false;

        convert_to_applicable_state(
            &balancer_desired_state,
            &agent_controller_pool,
            &balancer_applicable_state_holder,
            &mut is_converted_to_applicable_state,
        )
        .await
        .unwrap();

        assert!(is_converted_to_applicable_state);
        assert_eq!(
            balancer_applicable_state_holder.get_agent_desired_state(),
            Some(balancer_desired_state.to_agent_desired_state())
        );
    }

    #[tokio::test]
    async fn convert_to_applicable_state_errors_when_agent_message_receiver_dropped() {
        let balancer_desired_state = BalancerDesiredState::default();
        let agent_controller_pool = AgentControllerPool::default();

        agent_controller_pool
            .register_agent_controller(
                "agent-test".to_owned(),
                agent_controller_with_dropped_receiver(),
            )
            .unwrap();

        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();
        let mut is_converted_to_applicable_state = false;

        let result = convert_to_applicable_state(
            &balancer_desired_state,
            &agent_controller_pool,
            &balancer_applicable_state_holder,
            &mut is_converted_to_applicable_state,
        )
        .await;

        assert!(result.err().is_some());
        assert!(!is_converted_to_applicable_state);
    }

    #[tokio::test]
    async fn try_convert_to_applicable_state_keeps_flag_false_when_agent_send_fails() {
        let balancer_desired_state = BalancerDesiredState::default();
        let agent_controller_pool = AgentControllerPool::default();

        agent_controller_pool
            .register_agent_controller(
                "agent-test".to_owned(),
                agent_controller_with_dropped_receiver(),
            )
            .unwrap();

        let balancer_applicable_state_holder = BalancerApplicableStateHolder::default();
        let mut is_converted_to_applicable_state = false;

        try_convert_to_applicable_state(
            &balancer_desired_state,
            &agent_controller_pool,
            &balancer_applicable_state_holder,
            &mut is_converted_to_applicable_state,
        )
        .await;

        assert!(!is_converted_to_applicable_state);
    }
}
