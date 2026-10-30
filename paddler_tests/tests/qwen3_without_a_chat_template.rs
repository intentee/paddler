#![cfg(feature = "tests_that_use_llms")]

use std::path::PathBuf;

use tempfile::TempDir;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedded_chat_template_key::EMBEDDED_CHAT_TEMPLATE_KEY;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::write_patched_gguf::write_patched_gguf;

pub struct Qwen3WithoutAChatTemplate {
    model_directory: TempDir,
}

impl Qwen3WithoutAChatTemplate {
    pub fn write() -> Self {
        let qwen3_without_a_chat_template = Self {
            model_directory: TempDir::new().expect("a temporary model directory must be creatable"),
        };

        write_patched_gguf(
            &qwen3_0_6b()
                .cached_path()
                .expect("Qwen3 must be in the Hugging Face cache"),
            &qwen3_without_a_chat_template.model_path(),
            |gguf_context| {
                gguf_context
                    .remove_key(EMBEDDED_CHAT_TEMPLATE_KEY)
                    .expect("Qwen3 must embed a chat template to remove");
            },
        )
        .expect("the model without a chat template must be writable");

        qwen3_without_a_chat_template
    }

    pub fn desired_state(&self) -> BalancerDesiredState {
        BalancerDesiredState {
            model: agent_desired_model_at(&self.model_path())
                .expect("the model without a chat template must be at an absolute path"),
            ..qwen3_desired_state()
        }
    }

    pub fn model_path(&self) -> PathBuf {
        self.model_directory
            .path()
            .join("qwen3-without-a-chat-template.gguf")
    }
}
