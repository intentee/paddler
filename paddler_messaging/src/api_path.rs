pub struct ApiPath;

impl ApiPath {
    #[must_use]
    pub fn agent_chat_template_override(agent_id: &str) -> String {
        format!("/api/v1/agent/{agent_id}/chat_template_override")
    }

    #[must_use]
    pub fn agent_model_metadata(agent_id: &str) -> String {
        format!("/api/v1/agent/{agent_id}/model_metadata")
    }

    #[must_use]
    pub fn agent_socket(agent_id: &str) -> String {
        format!("/api/v1/agent_socket/{agent_id}")
    }
}
