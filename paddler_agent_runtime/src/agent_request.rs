use tokio::sync::mpsc;

use paddler_agent_status::slot_guard::SlotGuard;

pub struct AgentRequest<TParams, TResponse> {
    pub params: TParams,
    pub response_tx: mpsc::UnboundedSender<TResponse>,
    pub slot_guard: SlotGuard,
    pub stop_rx: mpsc::UnboundedReceiver<()>,
}
