use std::process::Command;

use crate::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;

#[test]
fn paddler_agent_rejects_more_slots_than_llama_cpp_can_serve() {
    let agent_output = Command::new(env!("CARGO_BIN_EXE_paddler"))
        .arg("agent")
        .arg("--management-addr")
        .arg("127.0.0.1:1")
        .arg("--slots")
        .arg("257")
        .output()
        .expect("paddler agent must run");

    assert_eq!(agent_output.status.code(), Some(CLAP_USAGE_ERROR_EXIT_CODE));
    assert!(agent_output.stdout.is_empty());
}
