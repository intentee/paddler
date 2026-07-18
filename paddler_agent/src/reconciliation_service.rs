use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_desired_state_converter::AgentDesiredStateConverter;
use crate::agent_issue_fix::AgentIssueFix;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use paddler_state_conversion::converts_to_applicable_state::ConvertsToApplicableState as _;

async fn convert_to_applicable_state(
    cancellation_token: &CancellationToken,
    agent_desired_state: Option<&AgentDesiredState>,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    agent_applicable_state_holder: &AgentApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) -> Result<()> {
    let applicable_state = match agent_desired_state {
        None => None,
        Some(agent_desired_state) => Some(
            AgentDesiredStateConverter {
                cancellation_token: cancellation_token.clone(),
                slot_aggregated_status: slot_aggregated_status.clone(),
            }
            .to_applicable_state(agent_desired_state.clone())
            .await?,
        ),
    };

    slot_aggregated_status.set_uses_chat_template_override(
        applicable_state
            .as_ref()
            .is_some_and(|applicable_state| applicable_state.chat_template_override.is_some()),
    );
    slot_aggregated_status.register_fix(&AgentIssueFix::ModelStateIsReconciled);
    agent_applicable_state_holder.set_agent_applicable_state(applicable_state)?;
    *is_converted_to_applicable_state = true;

    Ok(())
}

async fn try_convert_to_applicable_state(
    cancellation_token: &CancellationToken,
    agent_desired_state: Option<&AgentDesiredState>,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    agent_applicable_state_holder: &AgentApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) {
    if let Err(err) = convert_to_applicable_state(
        cancellation_token,
        agent_desired_state,
        slot_aggregated_status,
        agent_applicable_state_holder,
        is_converted_to_applicable_state,
    )
    .await
        && !cancellation_token.is_cancelled()
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
                            &shutdown,
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
                        &shutdown,
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

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::inference_parameters::InferenceParameters;

    use super::*;

    #[tokio::test]
    async fn a_cancelled_conversion_leaves_the_state_unconverted() {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
        let agent_applicable_state_holder = AgentApplicableStateHolder::default();
        let mut is_converted_to_applicable_state = false;

        let desired_state = AgentDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent(
                "/paddler-nonexistent-model-for-cancellation.gguf".to_owned(),
            ),
            multimodal_projection: AgentDesiredModel::None,
        };

        try_convert_to_applicable_state(
            &cancellation_token,
            Some(&desired_state),
            &slot_aggregated_status,
            &agent_applicable_state_holder,
            &mut is_converted_to_applicable_state,
        )
        .await;

        assert!(!is_converted_to_applicable_state);
    }

    #[tokio::test]
    async fn flag_stays_false_when_set_agent_applicable_state_fails() {
        let holder = AgentApplicableStateHolder::default();
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
        let mut is_converted_to_applicable_state = false;

        let result = convert_to_applicable_state(
            &CancellationToken::new(),
            None,
            &slot_aggregated_status,
            &holder,
            &mut is_converted_to_applicable_state,
        )
        .await;

        assert!(result.is_err());
        assert!(!is_converted_to_applicable_state);
    }
}
