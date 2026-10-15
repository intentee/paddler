use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
use crate::agent_desired_state_converter::AgentDesiredStateConverter;
use crate::desired_state_reconciliation::DesiredStateReconciliation;

#[derive(Clone)]
pub struct DesiredStateReconciler {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub cache_dir: CacheDir,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl DesiredStateReconciler {
    pub async fn reconcile(
        &self,
        cancellation_token: &CancellationToken,
        agent_desired_state: AgentDesiredState,
    ) -> DesiredStateReconciliation {
        let conversion = AgentDesiredStateConverter {
            cache_dir: self.cache_dir.clone(),
            cancellation_token: cancellation_token.clone(),
            slot_aggregated_status: self.slot_aggregated_status.clone(),
        }
        .convert(&agent_desired_state)
        .await;

        match conversion {
            Ok(AgentDesiredStateConversion::Converted(applicable_state)) => {
                self.agent_applicable_state_holder
                    .set_agent_applicable_state(applicable_state);

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
}
