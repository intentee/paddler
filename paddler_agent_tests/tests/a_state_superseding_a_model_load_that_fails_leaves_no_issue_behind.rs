use paddler_agent_tests::settings_whose_model_cannot_load::settings_whose_model_cannot_load;
use paddler_agent_tests::startup_superseded_while_its_blocking_work_waits::StartupSupersededWhileItsBlockingWorkWaits;
use paddler_agent_tests::state_whose_model_cannot_load::state_whose_model_cannot_load;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[test]
fn a_state_superseding_a_model_load_that_fails_leaves_no_issue_behind() {
    assert!(
        !StartupSupersededWhileItsBlockingWorkWaits {
            superseded_state: state_whose_model_cannot_load(),
        }
        .run()
        .expect("the superseded startup must run to its end")
        .has_issue(&AgentIssue::ModelCannotBeLoaded(ModelPath::from(
            settings_whose_model_cannot_load().model_path.as_path()
        )))
    );
}
