use std::sync::Arc;
use std::time::Duration;

use log::warn;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tokio_util::sync::DropGuard;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::dispatchable_request::DispatchableRequest;
use crate::dispatched_agent_relay::DispatchedAgentRelay;
use crate::relay_failure::RelayFailure;

pub struct AwaitingAgentRelay<TParams> {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub dispatchable_request: DispatchableRequest<TParams>,
    pub inference_item_timeout: Duration,
    pub shutdown: CancellationToken,
}

impl<TParams> AwaitingAgentRelay<TParams>
where
    TParams: AgentStreamingRequest,
{
    pub async fn dispatch(
        self,
        settlement_guard: DropGuard,
    ) -> Result<DispatchedAgentRelay<TParams::Response>, RelayFailure> {
        let Self {
            buffered_request_manager,
            dispatchable_request:
                DispatchableRequest {
                    dispatch_requirement,
                    params,
                },
            inference_item_timeout,
            shutdown,
        } = self;

        let buffered_request_agent_wait_result = select! {
            biased;

            () = shutdown.cancelled() => return Err(RelayFailure::BalancerShuttingDown),
            buffered_request_agent_wait_result = buffered_request_manager.wait_for_available_agent(dispatch_requirement) => {
                buffered_request_agent_wait_result
            }
        };

        match buffered_request_agent_wait_result {
            BufferedRequestAgentWaitResult::Found(dispatched_agent) => {
                Ok(DispatchedAgentRelay::start(
                    dispatched_agent,
                    params,
                    inference_item_timeout,
                    settlement_guard.disarm(),
                    shutdown,
                ))
            }
            BufferedRequestAgentWaitResult::BufferOverflow => {
                warn!("Too many buffered requests, dropping a request");

                Err(RelayFailure::BufferOverflow)
            }
            BufferedRequestAgentWaitResult::Timeout => {
                warn!("A buffered request timed out waiting for an available slot");

                Err(RelayFailure::BufferedRequestTimedOut)
            }
        }
    }
}
