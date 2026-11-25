use std::sync::Arc;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;

pub enum EstablishedCapabilitiesPlan {
    Establish,
    Keep(Arc<EstablishedModelCapabilities>),
}
