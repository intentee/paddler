use std::sync::Arc;

use crate::buffered_request_count_guard::BufferedRequestCountGuard;
use crate::buffered_request_counter::BufferedRequestCounter;
use crate::dispatch_position::DispatchPosition;
use crate::dispatch_requirement::DispatchRequirement;
use crate::dispatch_waiting_line::DispatchWaitingLine;
use crate::waiting_line_admission::WaitingLineAdmission;

pub struct WaitingLinePlace<'line> {
    pub buffered_request_count_guard: BufferedRequestCountGuard,
    pub dispatch_waiting_line: &'line DispatchWaitingLine,
    pub ticket: u64,
}

impl<'line> WaitingLinePlace<'line> {
    #[must_use]
    pub fn join_when_admitted(
        dispatch_waiting_line: &'line DispatchWaitingLine,
        dispatch_requirement: DispatchRequirement,
        buffered_request_counter: &Arc<BufferedRequestCounter>,
    ) -> Option<Self> {
        dispatch_waiting_line
            .admit(dispatch_requirement, buffered_request_counter)
            .map(
                |WaitingLineAdmission {
                     buffered_request_count_guard,
                     ticket,
                 }| Self {
                    buffered_request_count_guard,
                    dispatch_waiting_line,
                    ticket,
                },
            )
    }

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
