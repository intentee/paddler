use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::requested_capability::RequestedCapability;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterLacksCapability {
    pub requested_capability: RequestedCapability,
}

impl ClusterLacksCapability {
    #[must_use]
    pub fn into_jsonrpc_error(self) -> JsonRpcError {
        JsonRpcError {
            code: 503,
            description: self.to_string(),
        }
    }
}

impl Display for ClusterLacksCapability {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "No agent serves or is loading a model that supports {}",
            self.requested_capability
        )
    }
}
