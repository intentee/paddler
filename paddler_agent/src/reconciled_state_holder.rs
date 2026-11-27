use tokio::sync::watch;

use crate::reconciled_state::ReconciledState;
use crate::reconciliation_progress::ReconciliationProgress;

#[derive(Default)]
pub struct ReconciledStateHolder {
    reconciliation_progress_tx: watch::Sender<ReconciliationProgress>,
}

impl ReconciledStateHolder {
    #[must_use]
    pub fn get_reconciliation_progress(&self) -> ReconciliationProgress {
        self.reconciliation_progress_tx.borrow().clone()
    }

    pub fn set_reconciled_state(&self, reconciled_state: ReconciledState) {
        self.reconciliation_progress_tx
            .send_replace(ReconciliationProgress::Reconciled(reconciled_state));
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<ReconciliationProgress> {
        let mut reconciliation_progress_rx = self.reconciliation_progress_tx.subscribe();

        reconciliation_progress_rx.mark_changed();

        reconciliation_progress_rx
    }
}
