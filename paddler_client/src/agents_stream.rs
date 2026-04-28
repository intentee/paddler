use std::pin::Pin;

use futures_util::Stream;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

use crate::Result;

pub type AgentsStream = Pin<Box<dyn Stream<Item = Result<AgentControllerPoolSnapshot>> + Send>>;
