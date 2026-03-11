use std::sync::LazyLock;
use std::time::Duration;

use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::managed_balancer::ManagedBalancerParams;

pub mod managed_agent;
pub mod managed_balancer;

pub const BALANCER_MANAGEMENT_ADDR: &str = "127.0.0.1:8060";
pub const BALANCER_INFERENCE_ADDR: &str = "127.0.0.1:8061";
pub const BALANCER_OPENAI_ADDR: &str = "127.0.0.1:8062";
pub const PADDLER_BINARY_PATH: &str = "../target/debug/paddler";
pub const TIMEOUT: Duration = Duration::from_secs(3);
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub static AGENT_DESIRED_MODEL: LazyLock<AgentDesiredModel> = LazyLock::new(|| {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
        repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
        revision: "main".to_string(),
    })
});

pub fn balancer_params(
    management_addr: &str,
    inference_addr: &str,
    state_database_path: &str,
    max_buffered_requests: i32,
    buffered_request_timeout: Duration,
) -> ManagedBalancerParams {
    ManagedBalancerParams {
        buffered_request_timeout,
        compat_openai_addr: None,
        inference_addr: inference_addr.to_string(),
        management_addr: management_addr.to_string(),
        max_buffered_requests,
        state_database_path: state_database_path.to_string(),
    }
}

pub fn balancer_params_with_openai(
    management_addr: &str,
    inference_addr: &str,
    openai_addr: &str,
    state_database_path: &str,
    max_buffered_requests: i32,
    buffered_request_timeout: Duration,
) -> ManagedBalancerParams {
    ManagedBalancerParams {
        buffered_request_timeout,
        compat_openai_addr: Some(openai_addr.to_string()),
        inference_addr: inference_addr.to_string(),
        management_addr: management_addr.to_string(),
        max_buffered_requests,
        state_database_path: state_database_path.to_string(),
    }
}
