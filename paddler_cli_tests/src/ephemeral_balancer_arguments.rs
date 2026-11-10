use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;

#[must_use]
pub fn ephemeral_balancer_arguments() -> [String; 5] {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();

    [
        "balancer".to_owned(),
        "--inference-addr".to_owned(),
        ephemeral_loopback_addr.clone(),
        "--management-addr".to_owned(),
        ephemeral_loopback_addr,
    ]
}
