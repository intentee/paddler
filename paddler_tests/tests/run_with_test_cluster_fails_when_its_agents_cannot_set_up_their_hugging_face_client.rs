#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

use paddler_test_cluster_harness::failed_run_exit_code::FAILED_RUN_EXIT_CODE;

const USER_AGENT_ORIGIN_WITH_A_LINE_BREAK: &str = "paddler\ntests";

#[test]
fn run_with_test_cluster_fails_when_its_agents_cannot_set_up_their_hugging_face_client() {
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args(["qwen3-0-6b", "--", "true"])
            .env(
                "HF_HUB_USER_AGENT_ORIGIN",
                USER_AGENT_ORIGIN_WITH_A_LINE_BREAK
            )
            .status()
            .expect("the test cluster runner must run")
            .code(),
        Some(FAILED_RUN_EXIT_CODE)
    );
}
