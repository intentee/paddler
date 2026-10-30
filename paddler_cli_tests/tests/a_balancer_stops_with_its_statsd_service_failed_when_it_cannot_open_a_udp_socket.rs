use std::error::Error as _;
use std::io;

use libc::EPERM;
use libc::SOCK_CLOEXEC;
use libc::SOCK_DGRAM;
use libc::SOCK_NONBLOCK;
use libc::SYS_socket;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;
use trzcina::ServiceShutdownError;
use trzcina::ServiceShutdownOutcome;
use trzcina::ServiceShutdownOutcomeWithServiceName;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::longer_than_any_test_run::LONGER_THAN_ANY_TEST_RUN;
use paddler_tests::in_process_balancer_runner_params::in_process_balancer_runner_params;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_stops_with_its_statsd_service_failed_when_it_cannot_open_a_udp_socket() {
    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_socket,
            rules: vec![
                SeccompRule::new(vec![
                    SeccompCondition::new(
                        1,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::MaskedEq(u64::from(
                            (!(SOCK_CLOEXEC | SOCK_NONBLOCK)).cast_unsigned(),
                        )),
                        u64::from(SOCK_DGRAM.cast_unsigned()),
                    )
                    .expect("the datagram socket condition must be valid"),
                ])
                .expect("the datagram socket rule must be valid"),
            ],
        }],
    }
    .apply_to_every_thread();

    let balancer_runner_params =
        in_process_balancer_runner_params(&ClusterParams::without_agents())
            .expect("the balancer parameters must be valid");
    let mut balancer_runner = BalancerRunner::start(BalancerRunnerParams {
        bootstrap_config: BalancerBootstrapConfig {
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                statsd_reporting_interval: LONGER_THAN_ANY_TEST_RUN,
            }),
            ..balancer_runner_params.bootstrap_config
        },
        ..balancer_runner_params
    })
    .await
    .expect("the balancer must bind its listeners");

    assert_eq!(
        balancer_runner
            .wait_for_completion()
            .await
            .expect_err("a balancer whose statsd service fails must stop with an error")
            .source()
            .map(ToString::to_string),
        Some(
            ServiceShutdownError::new(vec![ServiceShutdownOutcomeWithServiceName {
                name: "balancer::statsd_service",
                outcome: ServiceShutdownOutcome::Errored(
                    io::Error::from_raw_os_error(EPERM).into()
                ),
            }])
            .to_string()
        )
    );
}
