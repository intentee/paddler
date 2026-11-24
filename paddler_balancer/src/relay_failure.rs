use std::time::Duration;

use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

#[derive(Debug, Eq, PartialEq)]
pub enum RelayFailure {
    AgentDisconnected,
    BalancerShuttingDown,
    BufferOverflow,
    BufferedRequestTimedOut,
    ItemTimedOut { inference_item_timeout: Duration },
}

impl RelayFailure {
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::AgentDisconnected => "Agent controller connection closed".to_owned(),
            Self::BalancerShuttingDown => "balancer is shutting down".to_owned(),
            Self::BufferOverflow => "Buffered requests overflow".to_owned(),
            Self::BufferedRequestTimedOut => "Waiting for available slot timed out".to_owned(),
            Self::ItemTimedOut {
                inference_item_timeout,
            } => {
                let timeout_ms = inference_item_timeout.as_millis();

                format!(
                    "Inference timed out after {timeout_ms}ms waiting for next token. \
                    Increase --inference-item-timeout if the prompt requires longer processing."
                )
            }
        }
    }

    #[must_use]
    pub fn into_jsonrpc_error(self) -> JsonRpcError {
        JsonRpcError {
            code: match self {
                Self::AgentDisconnected => 502,
                Self::BalancerShuttingDown | Self::BufferOverflow => 503,
                Self::BufferedRequestTimedOut | Self::ItemTimedOut { .. } => 504,
            },
            description: self.description(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RelayFailure;

    #[test]
    fn every_relay_failure_reaches_the_client_as_its_jsonrpc_error() {
        for (relay_failure, expected_code) in [
            (RelayFailure::AgentDisconnected, 502),
            (RelayFailure::BalancerShuttingDown, 503),
            (RelayFailure::BufferOverflow, 503),
            (RelayFailure::BufferedRequestTimedOut, 504),
            (
                RelayFailure::ItemTimedOut {
                    inference_item_timeout: Duration::from_millis(250),
                },
                504,
            ),
        ] {
            assert_eq!(relay_failure.into_jsonrpc_error().code, expected_code);
        }
    }
}
