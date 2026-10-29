use std::path::Path;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
use paddler_model_source::model_source::ModelSource;

use crate::reconciliation_after_replacement::ReconciliationAfterReplacement;
use crate::reconciliation_replaced_before_its_worker_runs::ReconciliationReplacedBeforeItsWorkerRuns;

const MISSING_MODEL_PATH: &str = "/paddler-missing-model-of-a-replaced-desired-state.gguf";

#[test]
fn a_reconciliation_replaced_before_its_worker_runs_leaves_no_issue_behind() {
    let ReconciliationAfterReplacement {
        slot_aggregated_status,
        ..
    } = ReconciliationReplacedBeforeItsWorkerRuns {
        replaced_desired_state: AgentDesiredState {
            model: ModelSource::LocalToAgent(
                AgentLocalModelPath::from_absolute_path(Path::new(MISSING_MODEL_PATH))
                    .expect("the missing model path must be absolute"),
            )
            .into_agent_desired_model(),
            ..AgentDesiredState::default()
        },
    }
    .run();

    assert!(
        !slot_aggregated_status.has_issue(&AgentIssue::ModelFileDoesNotExist(ModelPath {
            model_path: MISSING_MODEL_PATH.to_owned(),
        }))
    );
}
