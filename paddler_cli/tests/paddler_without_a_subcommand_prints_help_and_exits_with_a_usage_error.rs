use std::process::Command;

use crate::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;

#[test]
fn paddler_without_a_subcommand_prints_help_and_exits_with_a_usage_error() {
    let help_output = Command::new(env!("CARGO_BIN_EXE_paddler"))
        .arg("--help")
        .output()
        .expect("paddler --help must run");
    let bare_output = Command::new(env!("CARGO_BIN_EXE_paddler"))
        .output()
        .expect("paddler without arguments must run");

    assert_eq!(bare_output.status.code(), Some(CLAP_USAGE_ERROR_EXIT_CODE));
    assert!(bare_output.stdout.is_empty());
    assert_eq!(bare_output.stderr, help_output.stdout);
}
