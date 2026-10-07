use std::net::TcpListener;
use std::process::Command;

const FAILED_MAIN_EXIT_CODE: i32 = 1;

#[test]
fn paddler_balancer_decision_exits_with_a_failure_when_its_management_address_is_taken() {
    let taken_listener = TcpListener::bind("127.0.0.1:0")
        .expect("a loopback listener must bind to an ephemeral port");
    let taken_address = taken_listener
        .local_addr()
        .expect("the bound listener must report its address");

    let balancer_output = Command::new(env!("CARGO_BIN_EXE_paddler"))
        .arg("balancer")
        .arg("decision")
        .arg("--inference-addr")
        .arg("127.0.0.1:0")
        .arg("--management-addr")
        .arg(taken_address.to_string())
        .output()
        .expect("paddler balancer must run");

    assert_eq!(balancer_output.status.code(), Some(FAILED_MAIN_EXIT_CODE));
    assert!(balancer_output.stdout.is_empty());
}
