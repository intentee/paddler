use tokio::sync::watch;

use paddler_messaging::desired_state_version::DesiredStateVersion;

use crate::reconciled_state::ReconciledState;

#[derive(Default)]
pub struct ReconciledStateHolder {
    reconciled_state_tx: watch::Sender<ReconciledState>,
}

impl ReconciledStateHolder {
    pub fn acknowledge(&self, desired_state_version: DesiredStateVersion) {
        self.reconciled_state_tx.send_modify(|reconciled_state| {
            reconciled_state.desired_state_version = desired_state_version;
        });
    }

    #[must_use]
    pub fn get_reconciled_state(&self) -> ReconciledState {
        self.reconciled_state_tx.borrow().clone()
    }

    pub fn set_reconciled_state(&self, reconciled_state: ReconciledState) {
        self.reconciled_state_tx.send_replace(reconciled_state);
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<ReconciledState> {
        let mut reconciled_state_rx = self.reconciled_state_tx.subscribe();

        reconciled_state_rx.mark_changed();

        reconciled_state_rx
    }
}
