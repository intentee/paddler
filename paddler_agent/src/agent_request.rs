use tokio::sync::mpsc;

use crate::slot_guard::SlotGuard;

pub struct AgentRequest<TParams, TResponse> {
    pub params: TParams,
    pub response_tx: mpsc::UnboundedSender<TResponse>,
    pub slot_guard: SlotGuard,
    pub stop_rx: mpsc::UnboundedReceiver<()>,
}
