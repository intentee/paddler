#![cfg(target_os = "linux")]

use seccompiler::SeccompRule;

pub struct RefusedSyscall {
    pub syscall: i64,
    pub rules: Vec<SeccompRule>,
}
