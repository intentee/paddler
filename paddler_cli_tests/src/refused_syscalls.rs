use std::env::consts::ARCH;

use anyhow::Context as _;
use anyhow::Result;
use seccompiler::BpfProgram;
use seccompiler::SeccompAction;
use seccompiler::SeccompFilter;
use seccompiler::TargetArch;
use seccompiler::apply_filter_all_threads;

use crate::refused_syscall::RefusedSyscall;

pub struct RefusedSyscalls {
    pub errno: i32,
    pub refused_syscalls: Vec<RefusedSyscall>,
}

impl RefusedSyscalls {
    /// # Errors
    ///
    /// Fails when seccompiler does not support the architecture the tests run on, when the refusing filter is invalid or does not compile into BPF, and when it cannot apply to every thread of the test process.
    pub fn apply_to_every_thread(self) -> Result<()> {
        let Self {
            errno,
            refused_syscalls,
        } = self;

        TargetArch::try_from(ARCH)
            .context("seccompiler does not support the architecture the tests run on")
            .and_then(|target_arch| {
                SeccompFilter::new(
                    refused_syscalls
                        .into_iter()
                        .map(|RefusedSyscall { syscall, rules }| (syscall, rules))
                        .collect(),
                    SeccompAction::Allow,
                    SeccompAction::Errno(errno.cast_unsigned()),
                    target_arch,
                )
                .context("the refusing filter is invalid")
            })
            .and_then(|refusing_filter| {
                BpfProgram::try_from(refusing_filter)
                    .context("the refusing filter does not compile into BPF")
            })
            .and_then(|refusing_filter| {
                apply_filter_all_threads(&refusing_filter).context(
                    "the refusing filter does not apply to every thread of the test process",
                )
            })
    }
}
