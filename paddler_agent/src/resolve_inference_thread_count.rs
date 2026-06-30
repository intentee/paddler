use std::cmp::max;

#[must_use]
pub fn resolve_inference_thread_count() -> i32 {
    let logical_cpu_count = i32::try_from(num_cpus::get()).unwrap_or(i32::MAX);

    max(2, logical_cpu_count / 2)
}
