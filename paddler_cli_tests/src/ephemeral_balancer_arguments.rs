use paddler_cluster_harness::ephemeral_loopback_addr::ephemeral_loopback_addr;

#[must_use]
pub fn ephemeral_balancer_arguments() -> [String; 5] {
    let ephemeral_loopback_addr = ephemeral_loopback_addr().to_string();

    [
        "balancer".to_owned(),
        "--inference-addr".to_owned(),
        ephemeral_loopback_addr.clone(),
        "--management-addr".to_owned(),
        ephemeral_loopback_addr,
    ]
}
