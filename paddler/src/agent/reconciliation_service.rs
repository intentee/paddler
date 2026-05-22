use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_types::agent_desired_state::AgentDesiredState;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_issue_fix::AgentIssueFix;
use crate::converts_to_applicable_state::ConvertsToApplicableState as _;
use crate::slot_aggregated_status::SlotAggregatedStatus;

async fn convert_to_applicable_state(
    agent_desired_state: Option<&AgentDesiredState>,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    agent_applicable_state_holder: &AgentApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) -> Result<()> {
    let applicable_state = match agent_desired_state {
        None => None,
        Some(agent_desired_state) => Some(
            agent_desired_state
                .to_applicable_state(slot_aggregated_status.clone())
                .await?,
        ),
    };

    *is_converted_to_applicable_state = true;
    slot_aggregated_status.set_uses_chat_template_override(
        applicable_state
            .as_ref()
            .is_some_and(|applicable_state| applicable_state.chat_template_override.is_some()),
    );
    slot_aggregated_status.register_fix(&AgentIssueFix::ModelStateIsReconciled);
    agent_applicable_state_holder.set_agent_applicable_state(applicable_state)
}

async fn try_convert_to_applicable_state(
    agent_desired_state: Option<&AgentDesiredState>,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    agent_applicable_state_holder: &AgentApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) {
    if let Err(err) = convert_to_applicable_state(
        agent_desired_state,
        slot_aggregated_status,
        agent_applicable_state_holder,
        is_converted_to_applicable_state,
    )
    .await
    {
        error!("Failed to convert to applicable state: {err}");
    }
}

pub struct ReconciliationService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state: Option<AgentDesiredState>,
    pub agent_desired_state_rx: mpsc::UnboundedReceiver<AgentDesiredState>,
    pub is_converted_to_applicable_state: bool,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "agent::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
            agent_applicable_state_holder,
            mut agent_desired_state,
            mut agent_desired_state_rx,
            mut is_converted_to_applicable_state,
            slot_aggregated_status,
        } = *self;

        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if !is_converted_to_applicable_state {
                        try_convert_to_applicable_state(
                            agent_desired_state.as_ref(),
                            &slot_aggregated_status,
                            &agent_applicable_state_holder,
                            &mut is_converted_to_applicable_state,
                        ).await;
                    }
                },
                next_agent_desired_state = agent_desired_state_rx.recv() => {
                    is_converted_to_applicable_state = false;
                    agent_desired_state = if let Some(next) = next_agent_desired_state {
                        Some(next)
                    } else {
                        error!("Agent desired state channel closed, stopping reconciliation service.");
                        break Ok(())
                    };
                    try_convert_to_applicable_state(
                        agent_desired_state.as_ref(),
                        &slot_aggregated_status,
                        &agent_applicable_state_holder,
                        &mut is_converted_to_applicable_state,
                    ).await;
                }
            }
        }
    }
}
