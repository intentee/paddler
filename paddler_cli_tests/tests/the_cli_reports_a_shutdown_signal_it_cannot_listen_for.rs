use std::io;

use libc::EPERM;
use libc::SIGTERM;
use libc::SYS_rt_sigaction;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;

use paddler_cli::run;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

#[test]
fn the_cli_reports_a_shutdown_signal_it_cannot_listen_for() {
    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_rt_sigaction,
            rules: vec![
                SeccompRule::new(vec![
                    SeccompCondition::new(
                        0,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        u64::from(SIGTERM.cast_unsigned()),
                    )
                    .expect("the termination signal condition must be valid"),
                ])
                .expect("the termination signal rule must be valid"),
            ],
        }],
    }
    .apply_to_every_thread();

    let cli_error = run().expect_err("a CLI that cannot listen for SIGTERM must not start");

    assert_eq!(cli_error.to_string(), "failed to listen for SIGTERM");
    assert_eq!(
        cli_error
            .root_cause()
            .downcast_ref::<io::Error>()
            .and_then(io::Error::raw_os_error),
        Some(EPERM)
    );
}
