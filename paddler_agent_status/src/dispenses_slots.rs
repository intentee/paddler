use std::num::NonZeroU16;

pub trait DispensesSlots {
    fn release_slots(&self, slots: NonZeroU16);
    fn take_slots(&self, slots: NonZeroU16);
}
