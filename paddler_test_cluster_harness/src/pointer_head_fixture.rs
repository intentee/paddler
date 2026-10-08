use paddler_messaging::agent_desired_model::AgentDesiredModel;

#[must_use]
pub fn pointer_head_fixture(fixture_file_name: &str) -> AgentDesiredModel {
    AgentDesiredModel::LocalToAgent(format!(
        "{}/../fixtures/{fixture_file_name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}
