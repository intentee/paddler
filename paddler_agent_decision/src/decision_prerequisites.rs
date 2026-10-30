use std::path::PathBuf;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;

use crate::decision_error::DecisionError;
use crate::loaded_pointer_head::LoadedPointerHead;

pub struct DecisionPrerequisites {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pointer_head_path: PathBuf,
}

impl DecisionPrerequisites {
    pub fn load_pointer_head(self) -> Result<LoadedPointerHead, DecisionError> {
        self.require_enough_slots()?;

        match PointerHead::load(&self.pointer_head_path) {
            Ok(pointer_head) => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::PointerHeadIsLoaded(ModelPath::from(
                        self.pointer_head_path.as_path(),
                    )));

                Ok(LoadedPointerHead {
                    pointer_head,
                    pointer_head_path: self.pointer_head_path,
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

    fn require_enough_slots(&self) -> Result<(), DecisionError> {
        let desired_slots = self
            .inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;

        if desired_slots >= DECISION_SLOTS_MINIMUM.get() {
            return Ok(());
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
