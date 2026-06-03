pub trait DispensesSlots {
    fn release_slot(&self);
    fn take_slot(&self);
}
