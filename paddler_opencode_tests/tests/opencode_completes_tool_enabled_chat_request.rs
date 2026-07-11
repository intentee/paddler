#![cfg(all(feature = "tests_that_use_llms", feature = "tests_that_use_opencode"))]

use std::time::Duration;

use anyhow::Result;
use paddler_opencode_tests::opencode_binary_path::opencode_binary_path;
use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;
use paddler_opencode_tests::run_opencode::run_opencode;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const OPENCODE_RUN_TIMEOUT: Duration = Duration::from_mins(5);

#[tokio::test(flavor = "multi_thread")]
async fn opencode_completes_tool_enabled_chat_request() -> Result<()> {
    let binary_path = opencode_binary_path()?;

    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let api_base_url = cluster
        .balancer
        .addresses
        .compat_openai_base_url()?
        .join("v1")?;

    let project = OpenCodeTestProject::create(&api_base_url, "PADDLER-OPENCODE-MARKER".to_owned())?;

    let prompt = format!(
        "Read the file {} in this directory and reply with the exact marker value it contains.",
        project.marker_file_name()
    );

    let outcome = run_opencode(&binary_path, &project, &prompt, OPENCODE_RUN_TIMEOUT).await?;

    cluster.shutdown().await?;

    assert!(
        outcome.exit_success,
        "OpenCode did not finish successfully; its tool-carrying requests must be accepted by Paddler.\nstdout:\n{}\nstderr:\n{}",
        outcome.stdout, outcome.stderr
    );
    assert!(
        !outcome.stdout.trim().is_empty(),
        "OpenCode produced no output.\nstderr:\n{}",
        outcome.stderr
    );

    Ok(())
}
