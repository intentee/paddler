use libc::EPERM;
use libc::SOCK_CLOEXEC;
use libc::SOCK_DGRAM;
use libc::SOCK_NONBLOCK;
use libc::SYS_socket;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;
use trzcina::ServiceShutdownOutcome;
use trzcina::ServiceShutdownOutcomeWithServiceName;

use paddler_balancer::balancer_error::BalancerError;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::ephemeral_loopback_addr::ephemeral_loopback_addr;
use paddler_cluster_harness::longer_than_any_test_run::longer_than_any_test_run;
use paddler_refused_syscalls::refused_syscall::RefusedSyscall;
use paddler_refused_syscalls::refused_syscalls::RefusedSyscalls;

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
    .apply_to_every_thread()
    .expect("the refusing filter must apply to every thread of the test process");

    let balancer_runner_params = ClusterParams::without_agents()
        .in_process_balancer_runner_params()
        .expect("the balancer parameters must be valid");
    let mut balancer_runner = BalancerRunner::start(BalancerRunnerParams {
        bootstrap_config: BalancerBootstrapConfig {
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
                statsd_reporting_interval: longer_than_any_test_run(),
            }),
            ..balancer_runner_params.bootstrap_config
        },
        ..balancer_runner_params
    })
    .await
    .expect("the balancer must bind its listeners");

    assert!(matches!(
        balancer_runner
            .wait_for_completion()
            .await
            .expect_err("a balancer whose statsd service fails must stop with an error"),
        BootstrapError::ServiceRunFailed { source }
            if matches!(
                source.failed_outcomes(),
                [ServiceShutdownOutcomeWithServiceName {
                    name: "balancer::statsd_service",
                    outcome: ServiceShutdownOutcome::Errored(service_error),
                }] if matches!(
                    service_error.downcast_ref::<BalancerError>(),
                    Some(BalancerError::StatsdSocketUnbindable(socket_error))
                        if socket_error.raw_os_error() == Some(EPERM)
                )
            )
    ));
}
