use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_desired_state::AgentDesiredState;
use crate::agent_issue_fix::AgentIssueFix;
use crate::converts_to_applicable_state::ConvertsToApplicableState as _;
use crate::service::Service;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct ReconciliationService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state: Option<AgentDesiredState>,
    pub agent_desired_state_rx: mpsc::UnboundedReceiver<AgentDesiredState>,
    pub is_converted_to_applicable_state: bool,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl ReconciliationService {
    pub async fn convert_to_applicable_state(&mut self) -> Result<()> {
        let applicable_state = match &self.agent_desired_state {
            None => None,
            Some(agent_desired_state) => {
                agent_desired_state
                    .to_applicable_state(self.slot_aggregated_status.clone())
                    .await?
            }
        };

        self.is_converted_to_applicable_state = true;
        self.slot_aggregated_status.set_uses_chat_template_override(
            if let Some(applicable_state) = &applicable_state {
                applicable_state.chat_template_override.is_some()
            } else {
                false
            },
        );
        self.slot_aggregated_status
            .register_fix(AgentIssueFix::ModelStateIsReconciled);
        self.agent_applicable_state_holder
            .set_agent_applicable_state(applicable_state)
    }

    pub async fn try_convert_to_applicable_state(&mut self) {
        if let Err(err) = self.convert_to_applicable_state().await {
            error!("Failed to convert to applicable state: {err}");
        }
    }
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "agent::reconciliation_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.recv() => break Ok(()),
                _ = ticker.tick() => {
                    if !self.is_converted_to_applicable_state {
                        self.try_convert_to_applicable_state().await;
                    }
                },
                next_agent_desired_state = self.agent_desired_state_rx.recv() => {
                    self.is_converted_to_applicable_state = false;
                    self.agent_desired_state = match next_agent_desired_state {
                        Some(agent_desired_state) => Some(agent_desired_state),
                        None => {
                            error!("Agent desired state channel closed, stopping reconciliation service.");

                            break Ok(())
                        }
                    };
                    self.try_convert_to_applicable_state().await;
                }
            }
        }
    }
}
