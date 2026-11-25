use std::mem::Discriminant;
use std::mem::discriminant;

use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;

#[must_use]
pub fn desired_state_reconciled_discriminant() -> Discriminant<DesiredStateReconciliation> {
    discriminant(&DesiredStateReconciliation::Reconciled(Box::default()))
}
