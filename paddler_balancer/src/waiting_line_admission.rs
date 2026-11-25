use crate::buffered_request_count_guard::BufferedRequestCountGuard;

pub struct WaitingLineAdmission {
    pub buffered_request_count_guard: BufferedRequestCountGuard,
    pub ticket: u64,
}
