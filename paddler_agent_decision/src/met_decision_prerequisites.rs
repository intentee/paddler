use std::num::NonZeroU16;

use crate::loaded_pointer_head::LoadedPointerHead;

pub struct MetDecisionPrerequisites {
    pub decision_slots: NonZeroU16,
    pub loaded_pointer_head: LoadedPointerHead,
}
