use std::process::Command;

use crate::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;

#[test]
fn paddler_balancer_refuses_compatibility_flags_of_other_modes() {
    for (inference_mode, compatibility_flag) in [
        ("decision", "--compat-openai-addr"),
        ("embeddings", "--compat-openai-addr"),
        ("embeddings", "--compat-typesafe-addr"),
        ("text-generation", "--compat-typesafe-addr"),
    ] {
        let balancer_output = Command::new(env!("CARGO_BIN_EXE_paddler"))
            .arg("balancer")
            .arg(inference_mode)
            .arg(compatibility_flag)
            .arg("127.0.0.1:0")
            .output()
            .expect("paddler balancer must run");

        assert_eq!(
            balancer_output.status.code(),
            Some(CLAP_USAGE_ERROR_EXIT_CODE),
            "{inference_mode} must refuse {compatibility_flag}"
        );
        assert!(balancer_output.stdout.is_empty());
    }
}
