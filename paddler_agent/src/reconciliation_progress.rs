use crate::reconciled_state::ReconciledState;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ReconciliationProgress {
    #[default]
    NothingReconciled,
    Reconciled(ReconciledState),
}
