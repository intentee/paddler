use std::num::NonZeroU16;
use std::path::PathBuf;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

use crate::decision_error::DecisionError;
use crate::loaded_pointer_head::LoadedPointerHead;
use crate::met_decision_prerequisites::MetDecisionPrerequisites;

pub struct DecisionPrerequisites {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pointer_head_path: PathBuf,
}

impl DecisionPrerequisites {
    /// # Errors
    ///
    /// Returns [`DecisionError::SlotsInsufficient`] when the agent runs fewer slots than decisions need, and [`DecisionError::PointerHeadCannotBeLoaded`] when the pointer head file cannot be loaded.
    pub fn meet(self) -> Result<MetDecisionPrerequisites, DecisionError> {
        let decision_slots = self.require_enough_slots()?;

        match PointerHead::load(&self.pointer_head_path) {
            Ok(pointer_head) => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::PointerHeadIsLoaded(ModelPath::from(
                        self.pointer_head_path.as_path(),
                    )));

                Ok(MetDecisionPrerequisites {
                    decision_slots,
                    loaded_pointer_head: LoadedPointerHead {
                        pointer_head,
                        pointer_head_path: self.pointer_head_path,
                    },
                })
            }
            Err(source) => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .register_pipeline_issue(AgentIssue::PointerHeadCannotBeLoaded(
                        ModelPath::from(self.pointer_head_path.as_path()),
                    ));

                Err(DecisionError::PointerHeadCannotBeLoaded(source))
            }
        }
    }

    fn require_enough_slots(&self) -> Result<NonZeroU16, DecisionError> {
        let desired_slots = self
            .inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;

        if let Some(decision_slots) = NonZeroU16::new(desired_slots)
            .filter(|decision_slots| *decision_slots >= DECISION_SLOTS_MINIMUM)
        {
            return Ok(decision_slots);
        }

        self.inference_runtime_context
            .slot_aggregated_status
            .register_pipeline_issue(AgentIssue::SlotsInsufficientForDecisions(
                SlotsInsufficientForDecisionsParams {
                    desired_slots,
                    required_slots: DECISION_SLOTS_MINIMUM.get(),
                },
            ));

        Err(DecisionError::SlotsInsufficient {
            desired_slots,
            required_slots: DECISION_SLOTS_MINIMUM.get(),
        })
    }
}
