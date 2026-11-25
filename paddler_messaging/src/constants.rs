use std::num::NonZeroU16;
use std::time::Duration;

pub const API_PATH_AGENTS: &str = "/api/v1/agents";
pub const API_PATH_AGENTS_STREAM: &str = "/api/v1/agents/stream";
pub const API_PATH_BALANCER_APPLICABLE_STATE: &str = "/api/v1/balancer_applicable_state";
pub const API_PATH_BALANCER_DESIRED_STATE: &str = "/api/v1/balancer_desired_state";
pub const API_PATH_BUFFERED_REQUESTS: &str = "/api/v1/buffered_requests";
pub const API_PATH_BUFFERED_REQUESTS_STREAM: &str = "/api/v1/buffered_requests/stream";
pub const API_PATH_CONTINUE_FROM_CONVERSATION_HISTORY: &str =
    "/api/v1/continue_from_conversation_history";
pub const API_PATH_CONTINUE_FROM_RAW_PROMPT: &str = "/api/v1/continue_from_raw_prompt";
pub const API_PATH_DECIDE: &str = "/api/v1/decide";
pub const API_PATH_GENERATE_EMBEDDING_BATCH: &str = "/api/v1/generate_embedding_batch";
pub const API_PATH_HEALTH: &str = "/health";
pub const API_PATH_INFERENCE_SOCKET: &str = "/api/v1/inference_socket";
pub const API_PATH_METRICS: &str = "/metrics";
pub const ASSISTANT_ROLE: &str = "assistant";
pub const DECISION_SLOTS_MINIMUM: NonZeroU16 = NonZeroU16::new(2).unwrap();
pub const EMBEDDED_CHAT_TEMPLATE_KEY: &str = "tokenizer.chat_template";
pub const PING_INTERVAL: Duration = Duration::from_secs(3);
pub const SYSTEM_ROLE: &str = "system";
pub const UNANSWERED_PINGS_TOLERATED: u32 = 1;
pub const USER_ROLE: &str = "user";
