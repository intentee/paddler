use std::sync::Arc;

use crate::agent_controller::AgentController;

pub enum AgentSocketRegistration {
    AwaitingRegistration,
    Registered(Arc<AgentController>),
}
