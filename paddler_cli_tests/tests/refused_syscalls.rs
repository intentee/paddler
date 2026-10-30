#![cfg(target_os = "linux")]

use std::env::consts::ARCH;

use seccompiler::BpfProgram;
use seccompiler::SeccompAction;
use seccompiler::SeccompFilter;
use seccompiler::apply_filter_all_threads;

use crate::refused_syscall::RefusedSyscall;

pub struct RefusedSyscalls {
    pub errno: i32,
    pub refused_syscalls: Vec<RefusedSyscall>,
}

impl RefusedSyscalls {
    pub fn apply_to_every_thread(self) {
        let Self {
            errno,
            refused_syscalls,
        } = self;
        let refusing_filter: BpfProgram = SeccompFilter::new(
            refused_syscalls
                .into_iter()
                .map(|RefusedSyscall { syscall, rules }| (syscall, rules))
                .collect(),
            SeccompAction::Allow,
            SeccompAction::Errno(errno.cast_unsigned()),
            ARCH.try_into()
                .expect("seccompiler must support the architecture the tests run on"),
        )
        .expect("the refusing filter must be valid")
        .try_into()
        .expect("the refusing filter must compile into BPF");

        apply_filter_all_threads(&refusing_filter)
            .expect("the refusing filter must apply to every thread of the test process");
    }
}
