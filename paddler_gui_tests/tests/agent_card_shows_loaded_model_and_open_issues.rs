use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::message::Message;
use paddler_gui::ui::view_agent_card::view_agent_card;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

#[test]
fn an_unnamed_agent_with_a_loaded_model_and_open_issues_renders_each_section() -> Result<()> {
    let mut issues = BTreeSet::new();
    issues.insert(AgentIssue::ModelFileDoesNotExist(ModelPath {
        model_path: "/var/models/model.gguf".to_owned(),
    }));

    let snapshot = AgentControllerSnapshot {
        desired_slots_total: 4,
        download_current: 0,
        download_filename: None,
        download_total: 0,
        id: "unnamed-agent-id".to_owned(),
        issues,
        model_path: Some("/var/models/model.gguf".to_owned()),
        name: None,
        slots_processing: 1,
        slots_total: 4,
        state_application_status: AgentStateApplicationStatus::Applied,
        uses_chat_template_override: false,
    };

    let mut simulator = simulator(view_agent_card::<Message>(&snapshot));

    if simulator.find("model.gguf").is_err() {
        bail!("expected the model file name to render");
    }
    if simulator.find("Status: OK").is_err() {
        bail!("expected the status label to render");
    }
    if simulator.find("1 issues").is_err() {
        bail!("expected the issues count to render");
    }

    Ok(())
}
