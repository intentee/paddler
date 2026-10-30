use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::settings_whose_model_cannot_load::settings_whose_model_cannot_load;
use crate::startup_superseded_while_its_blocking_work_waits::StartupSupersededWhileItsBlockingWorkWaits;
use crate::state_whose_model_cannot_load::state_whose_model_cannot_load;

#[test]
fn a_state_superseding_a_model_load_that_fails_leaves_no_issue_behind() {
    assert!(
        !StartupSupersededWhileItsBlockingWorkWaits {
            superseded_state: state_whose_model_cannot_load(),
        }
        .run()
        .has_issue(&AgentIssue::ModelCannotBeLoaded(ModelPath::from(
            settings_whose_model_cannot_load().model_path.as_path()
        )))
    );
}
