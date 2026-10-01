#![cfg(all(feature = "tests_that_use_llms", feature = "tests_that_use_opencode"))]

use paddler_opencode_tests::opencode_binary_path::opencode_binary_path;
use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;
use paddler_opencode_tests::run_opencode::run_opencode;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3_5_and_context_size::start_cluster_with_qwen3_5_and_context_size;

const OPENCODE_CONTEXT_SIZE: u32 = 16384;

#[tokio::test(flavor = "multi_thread")]
async fn opencode_completes_tool_enabled_chat_request() {
    let binary_path = opencode_binary_path().expect("the opencode binary must be configured");

    let cluster = start_cluster_with_qwen3_5_and_context_size(
        vec![AgentConfig::single(1)],
        false,
        OPENCODE_CONTEXT_SIZE,
    )
    .await
    .expect("the cluster must start");

    let api_base_url = cluster
        .balancer
        .compat_openai_base_url()
        .expect("the OpenAI compatibility service must have a base URL")
        .join("v1")
        .expect("every concurrent request must succeed");

    let project = OpenCodeTestProject::create(&api_base_url, "PADDLER-OPENCODE-MARKER".to_owned())
        .expect("the opencode test project must be created");

    let prompt = format!(
        "Read the file {} in this directory and reply with the exact marker value it contains.",
        project.marker_file_name()
    );

    let outcome = run_opencode(&binary_path, &project, &prompt)
        .await
        .expect("opencode must run");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    assert!(
        outcome.exit_success,
        "OpenCode did not finish successfully; its tool-carrying requests must be accepted by Paddler.\nstdout:\n{}\nstderr:\n{}",
        outcome.stdout, outcome.stderr
    );
    assert!(
        outcome.stdout.contains(project.marker_contents()),
        "OpenCode did not report the marker it was asked to read; the tool loop must complete.\nstdout:\n{}\nstderr:\n{}",
        outcome.stdout,
        outcome.stderr
    );
}
