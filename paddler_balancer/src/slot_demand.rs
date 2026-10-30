use std::num::NonZeroU16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotDemand {
    Exactly(NonZeroU16),
    UpTo(u16),
}

impl SlotDemand {
    #[must_use]
    pub const fn minimum_slots(self) -> u16 {
        match self {
            Self::Exactly(slots) => slots.get(),
            Self::UpTo(_slots) => 1,
        }
    }

    #[must_use]
    pub fn reservable_from(self, free_slots: u64) -> Option<NonZeroU16> {
        match self {
            Self::Exactly(slots) => (free_slots >= u64::from(slots.get())).then_some(slots),
            Self::UpTo(slots) => NonZeroU16::new(
                u16::try_from(free_slots).map_or(slots, |free_slots| free_slots.min(slots)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::SlotDemand;

    const TWO_SLOTS: NonZeroU16 = NonZeroU16::new(2).unwrap();

    #[test]
    fn an_exact_demand_reserves_its_slots_when_they_are_free() {
        assert_eq!(
            SlotDemand::Exactly(TWO_SLOTS).reservable_from(3),
            Some(TWO_SLOTS)
        );
    }

    #[test]
    fn an_exact_demand_reserves_nothing_while_fewer_slots_are_free() {
        assert_eq!(SlotDemand::Exactly(TWO_SLOTS).reservable_from(1), None);
    }

    #[test]
    fn a_flexible_demand_reserves_every_free_slot_it_can_use() {
        assert_eq!(SlotDemand::UpTo(5).reservable_from(2), Some(TWO_SLOTS));
        assert_eq!(SlotDemand::UpTo(2).reservable_from(5), Some(TWO_SLOTS));
    }

    #[test]
    fn a_flexible_demand_reserves_nothing_while_no_slot_is_free() {
        assert_eq!(SlotDemand::UpTo(5).reservable_from(0), None);
    }
}
