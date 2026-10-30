use crate::buffered_request_count_guard::BufferedRequestCountGuard;
use crate::dispatch_position::DispatchPosition;
use crate::dispatch_waiting_line::DispatchWaitingLine;

pub struct WaitingLinePlace<'line> {
    pub buffered_request_count_guard: BufferedRequestCountGuard,
    pub dispatch_waiting_line: &'line DispatchWaitingLine,
    pub ticket: u64,
}

impl WaitingLinePlace<'_> {
    #[must_use]
    pub const fn dispatch_position(&self) -> DispatchPosition {
        DispatchPosition::Waiting {
            ticket: self.ticket,
        }
    }
}

impl Drop for WaitingLinePlace<'_> {
    fn drop(&mut self) {
        self.dispatch_waiting_line.leave(self.ticket);
    }
}
