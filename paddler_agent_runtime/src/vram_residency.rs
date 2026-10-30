use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::vram_occupancy::VramOccupancy;
use crate::vram_resident::VramResident;

#[derive(Clone)]
pub struct VramResidency {
    occupied: Arc<AtomicBool>,
    resident: VramResident,
}

impl VramResidency {
    #[must_use]
    pub fn vacant(resident: VramResident) -> Self {
        Self {
            occupied: Arc::new(AtomicBool::new(false)),
            resident,
        }
    }

    pub fn try_occupy(&self) -> Result<VramOccupancy, AgentRuntimeError> {
        VramOccupancy::claim(&self.occupied, self.resident)
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::inference_capability::InferenceCapability;

    use super::VramResidency;
    use crate::agent_runtime_error::AgentRuntimeError;
    use crate::vram_resident::VramResident;

    #[test]
    fn an_occupied_residency_refuses_a_second_occupant_naming_what_is_resident() {
        for (resident, refusal) in [
            (
                VramResident::LlamaContext(InferenceCapability::Embeddings),
                AgentRuntimeError::LlamaContextAlreadyResident {
                    inference_capability: InferenceCapability::Embeddings,
                },
            ),
            (
                VramResident::ModelWeights,
                AgentRuntimeError::ModelWeightsAlreadyResident,
            ),
            (
                VramResident::MultimodalProjection,
                AgentRuntimeError::MultimodalProjectionAlreadyResident,
            ),
        ] {
            let vram_residency = VramResidency::vacant(resident);
            let _first_occupancy = vram_residency.try_occupy().unwrap();

            assert_eq!(
                vram_residency
                    .try_occupy()
                    .err()
                    .map(|error| error.to_string()),
                Some(refusal.to_string())
            );
        }
    }

    #[test]
    fn dropping_the_occupancy_vacates_the_residency_for_its_clones() {
        let vram_residency = VramResidency::vacant(VramResident::ModelWeights);
        let cloned_vram_residency = vram_residency.clone();

        drop(vram_residency.try_occupy().unwrap());

        assert!(cloned_vram_residency.try_occupy().is_ok());
    }
}
