use std::collections::HashSet;

use anyhow::Result;
use paddler_tests::balancer_addresses::BalancerAddresses;

#[tokio::test(flavor = "multi_thread")]
async fn picks_three_distinct_ports_per_invocation() -> Result<()> {
    let addresses = BalancerAddresses::pick()?;

    let mut ports = HashSet::new();

    ports.insert(addresses.compat_openai.port());
    ports.insert(addresses.inference.port());
    ports.insert(addresses.management.port());

    assert_eq!(
        ports.len(),
        3,
        "expected 3 distinct ports inside a single BalancerAddresses, got {ports:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn parallel_invocations_never_produce_internal_collisions() -> Result<()> {
    let concurrent_picks = 64;

    let mut handles = Vec::with_capacity(concurrent_picks);

    for _ in 0..concurrent_picks {
        handles.push(tokio::task::spawn_blocking(BalancerAddresses::pick));
    }

    for join_handle in handles {
        let addresses = join_handle.await??;

        let mut ports = HashSet::new();

        ports.insert(addresses.compat_openai.port());
        ports.insert(addresses.inference.port());
        ports.insert(addresses.management.port());

        assert_eq!(
            ports.len(),
            3,
            "BalancerAddresses::pick returned a collision inside the triple: {ports:?}"
        );
    }

    Ok(())
}
