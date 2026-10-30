use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use parking_lot::Mutex;

use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;
use crate::buffered_request_counter::BufferedRequestCounter;
use crate::dispatch_position::DispatchPosition;
use crate::dispatch_requirement::DispatchRequirement;
use crate::requirements_waiting_ahead::RequirementsWaitingAhead;
use crate::waiting_line_place::WaitingLinePlace;

pub struct DispatchWaitingLine {
    next_ticket: AtomicU64,
    updates: Arc<AgentControllerPoolUpdates>,
    waiting_requirements: Mutex<BTreeMap<u64, DispatchRequirement>>,
}

impl DispatchWaitingLine {
    #[must_use]
    pub const fn new(updates: Arc<AgentControllerPoolUpdates>) -> Self {
        Self {
            next_ticket: AtomicU64::new(0),
            updates,
            waiting_requirements: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn join_when_admitted(
        &self,
        dispatch_requirement: DispatchRequirement,
        buffered_request_counter: &Arc<BufferedRequestCounter>,
    ) -> Option<WaitingLinePlace<'_>> {
        let mut waiting_requirements = self.waiting_requirements.lock();
        let buffered_request_count_guard = buffered_request_counter.try_admit()?;
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        waiting_requirements.insert(ticket, dispatch_requirement);

        Some(WaitingLinePlace {
            buffered_request_count_guard,
            dispatch_waiting_line: self,
            ticket,
        })
    }

    pub fn leave(&self, ticket: u64) {
        self.waiting_requirements.lock().remove(&ticket);
        self.updates.signal_waiting_line_change();
    }

    #[must_use]
    pub fn requirements_ahead_of(
        &self,
        dispatch_position: DispatchPosition,
    ) -> RequirementsWaitingAhead {
        RequirementsWaitingAhead {
            dispatch_requirements: self
                .waiting_requirements
                .lock()
                .iter()
                .filter(|(ticket, _dispatch_requirement)| dispatch_position.waits_behind(**ticket))
                .map(|(_ticket, dispatch_requirement)| *dispatch_requirement)
                .collect(),
        }
    }
}
