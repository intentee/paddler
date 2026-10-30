use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::vram_resident::VramResident;

#[derive(Debug)]
pub struct VramOccupancy {
    occupied: Arc<AtomicBool>,
}

impl VramOccupancy {
    pub fn claim(
        occupied: &Arc<AtomicBool>,
        resident: VramResident,
    ) -> Result<Self, AgentRuntimeError> {
        match occupied.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_was_vacant) => Ok(Self {
                occupied: occupied.clone(),
            }),
            Err(_already_occupied) => Err(resident.already_resident()),
        }
    }
}

impl Drop for VramOccupancy {
    fn drop(&mut self) {
        self.occupied.store(false, Ordering::Release);
    }
}
