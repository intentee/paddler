use paddler_agent_runtime::shuts_down::ShutsDown;

use crate::agent_error::AgentError;
use crate::model_serving::ModelServing;
use crate::served_model::ServedModel;

pub enum ReplacementOutcome {
    Cancelled {
        held: ModelServing,
    },
    Failed {
        error: AgentError,
        held: ModelServing,
    },
    NothingToServe,
    Serving(Box<ServedModel>),
}

impl ReplacementOutcome {
    #[must_use]
    pub fn into_model_serving(self) -> ModelServing {
        match self {
            Self::Cancelled { held } | Self::Failed { held, .. } => held,
            Self::NothingToServe => ModelServing::Idle,
            Self::Serving(served_model) => ModelServing::Serving(served_model),
        }
    }
}

impl ShutsDown for ReplacementOutcome {
    async fn shut_down(self) {
        self.into_model_serving().shut_down().await;
    }
}
