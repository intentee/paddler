pub struct ApiPath;

impl ApiPath {
    pub const AGENTS: &str = "/api/v1/agents";
    pub const AGENTS_STREAM: &str = "/api/v1/agents/stream";
    pub const BALANCER_APPLICABLE_STATE: &str = "/api/v1/balancer_applicable_state";
    pub const BALANCER_DESIRED_STATE: &str = "/api/v1/balancer_desired_state";
    pub const BUFFERED_REQUESTS: &str = "/api/v1/buffered_requests";
    pub const BUFFERED_REQUESTS_STREAM: &str = "/api/v1/buffered_requests/stream";
    pub const CONTINUE_FROM_CONVERSATION_HISTORY: &str =
        "/api/v1/continue_from_conversation_history";
    pub const CONTINUE_FROM_RAW_PROMPT: &str = "/api/v1/continue_from_raw_prompt";
    pub const DECIDE: &str = "/api/v1/decide";
    pub const GENERATE_EMBEDDING_BATCH: &str = "/api/v1/generate_embedding_batch";
    pub const HEALTH: &str = "/health";
    pub const INFERENCE_SOCKET: &str = "/api/v1/inference_socket";
    pub const METRICS: &str = "/metrics";

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
