use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_request_preparer::DecisionRequestPreparer;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::shuts_down::ShutsDown;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::DecideParams;

pub enum ServedDecisions {
    DecisionModelNotServed,
    NotServed,
    Served(SchedulerHandle<DecisionRequestPreparer>),
}

impl ServedDecisions {
    pub fn forward(
        &self,
        agent_name: Option<&str>,
        request: AgentRequest<DecideParams, DecisionResult>,
    ) {
        match self {
            Self::DecisionModelNotServed | Self::NotServed => {
                DecisionError::report_decisions_not_served(agent_name, &request.response_tx);
            }
            Self::Served(scheduler_handle) => scheduler_handle.request_preparer.prepare(request),
        }
    }

    #[must_use]
    pub const fn serving(&self) -> AgentDecisionsServing {
        match self {
            Self::DecisionModelNotServed => AgentDecisionsServing::DecisionModelNotServed,
            Self::NotServed => AgentDecisionsServing::NotServed,
            Self::Served(_) => AgentDecisionsServing::Served,
        }
    }
}

impl ShutsDown for ServedDecisions {
    async fn shut_down(self) {
        match self {
            Self::DecisionModelNotServed | Self::NotServed => {}
            Self::Served(scheduler_handle) => scheduler_handle.shut_down().await,
        }
    }
}
