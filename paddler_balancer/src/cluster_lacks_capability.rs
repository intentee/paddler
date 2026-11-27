use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use paddler_messaging::requested_capability::RequestedCapability;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterLacksCapability {
    pub requested_capability: RequestedCapability,
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
