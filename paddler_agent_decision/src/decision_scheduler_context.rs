use std::sync::Arc;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;

pub struct DecisionSchedulerContext {
    pub agent_name: Option<String>,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
}
