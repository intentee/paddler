use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
use crate::agent_desired_state_converter::AgentDesiredStateConverter;
use crate::desired_state_reconciliation::DesiredStateReconciliation;

pub struct DesiredStateReconciler {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl DesiredStateReconciler {
    pub async fn reconcile(
        &self,
        cancellation_token: &CancellationToken,
        agent_desired_state: AgentDesiredState,
    ) -> DesiredStateReconciliation {
        let conversion = AgentDesiredStateConverter {
            cancellation_token: cancellation_token.clone(),
            slot_aggregated_status: self.slot_aggregated_status.clone(),
        }
        .convert(&agent_desired_state)
        .await;

        match conversion {
            Ok(AgentDesiredStateConversion::Converted(applicable_state)) => {
                let uses_chat_template_override =
                    applicable_state.chat_template_override().is_some();

                self.agent_applicable_state_holder
                    .set_agent_applicable_state(applicable_state);
                self.slot_aggregated_status
                    .set_uses_chat_template_override(uses_chat_template_override);
                self.slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelStateIsReconciled);

                DesiredStateReconciliation::Reconciled
            }
            Ok(AgentDesiredStateConversion::Cancelled) => {
                DesiredStateReconciliation::Pending(Box::new(agent_desired_state))
            }
            Err(conversion_error) => {
                error!("Failed to convert to applicable state: {conversion_error}");

                DesiredStateReconciliation::Pending(Box::new(agent_desired_state))
            }
        }
    }

    pub async fn reconcile_replacement(
        &self,
        cancellation_token: &CancellationToken,
        agent_desired_state: AgentDesiredState,
    ) -> DesiredStateReconciliation {
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::DesiredStateIsReplaced);

        self.reconcile(cancellation_token, agent_desired_state)
            .await
    }
}
