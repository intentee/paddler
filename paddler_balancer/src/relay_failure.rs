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

    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

    use super::RelayFailure;

    #[test]
    fn every_relay_failure_reaches_the_client_as_its_jsonrpc_error() {
        for (relay_failure, expected_code, expected_description) in [
            (
                RelayFailure::AgentDisconnected,
                502,
                "Agent controller connection closed",
            ),
            (
                RelayFailure::BalancerShuttingDown,
                503,
                "balancer is shutting down",
            ),
            (
                RelayFailure::BufferOverflow,
                503,
                "Buffered requests overflow",
            ),
            (
                RelayFailure::BufferedRequestTimedOut,
                504,
                "Waiting for available slot timed out",
            ),
            (
                RelayFailure::ItemTimedOut {
                    inference_item_timeout: Duration::from_millis(250),
                },
                504,
                "Inference timed out after 250ms waiting for next token. Increase --inference-item-timeout if the prompt requires longer processing.",
            ),
        ] {
            assert_eq!(
                relay_failure.into_jsonrpc_error(),
                JsonRpcError {
                    code: expected_code,
                    description: expected_description.to_owned(),
                }
            );
        }
    }
}
