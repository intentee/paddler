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

    #[must_use]
    pub const fn agents() -> &'static str {
        "/api/v1/agents"
    }

    #[must_use]
    pub const fn agents_stream() -> &'static str {
        "/api/v1/agents/stream"
    }

    #[must_use]
    pub const fn balancer_applicable_state() -> &'static str {
        "/api/v1/balancer_applicable_state"
    }

    #[must_use]
    pub const fn balancer_desired_state() -> &'static str {
        "/api/v1/balancer_desired_state"
    }

    #[must_use]
    pub const fn buffered_requests() -> &'static str {
        "/api/v1/buffered_requests"
    }

    #[must_use]
    pub const fn buffered_requests_stream() -> &'static str {
        "/api/v1/buffered_requests/stream"
    }

    #[must_use]
    pub const fn continue_from_conversation_history() -> &'static str {
        "/api/v1/continue_from_conversation_history"
    }

    #[must_use]
    pub const fn continue_from_raw_prompt() -> &'static str {
        "/api/v1/continue_from_raw_prompt"
    }

    #[must_use]
    pub const fn decide() -> &'static str {
        "/api/v1/decide"
    }

    #[must_use]
    pub const fn generate_embedding_batch() -> &'static str {
        "/api/v1/generate_embedding_batch"
    }

    #[must_use]
    pub const fn health() -> &'static str {
        "/health"
    }

    #[must_use]
    pub const fn inference_socket() -> &'static str {
        "/api/v1/inference_socket"
    }

    #[must_use]
    pub const fn metrics() -> &'static str {
        "/metrics"
    }
}
