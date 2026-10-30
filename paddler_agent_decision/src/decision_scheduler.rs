use std::collections::VecDeque;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::info;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_messaging::decision_result::DecisionResult;

use crate::active_decision_request::ActiveDecisionRequest;
use crate::decision_advance::DecisionAdvance;
use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::prepared_decision_request::PreparedDecisionRequest;
use crate::reports_decision_outcome::ReportsDecisionOutcome as _;

pub struct DecisionScheduler<'model> {
    pub active_requests: VecDeque<ActiveDecisionRequest>,
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub capacity_ledger: DecisionCapacityLedger,
    pub llama_context: ResidentLlamaContext<'model>,
    pub pending_requests: VecDeque<PreparedDecisionRequest>,
    pub scheduler_context: DecisionSchedulerContext,
    pub scheduler_inbox: SchedulerInbox<PreparedDecisionRequest>,
}

impl DecisionScheduler<'_> {
    pub fn run(&mut self) {
        info!(
            "{:?}: decision scheduler started",
            self.scheduler_context.agent_name
        );

        while !self.agent_shutdown.is_cancelled() {
            if self.active_requests.is_empty() && self.pending_requests.is_empty() {
                if self.scheduler_inbox.shutdown_requested {
                    break;
                }

                self.wait_for_next_scheduler_message();
            } else {
                self.accept_new_scheduler_messages();
            }

            self.admit_pending_requests();
            self.advance_next_active_request();
        }

        self.reject_unfinished_requests();
        self.llama_context.synchronize();

        info!(
            "{:?}: decision scheduler stopped",
            self.scheduler_context.agent_name
        );
    }

    fn accept_new_scheduler_messages(&mut self) {
        while let Some(request) = self.scheduler_inbox.take_next_waiting_command() {
            self.pending_requests.push_back(request);
        }
    }

    fn admit_pending_requests(&mut self) {
        while let Some(mut request) = self.pending_requests.pop_front() {
            if request.decision_stop_rx.is_stop_requested() {
                DecisionResult::StopRequested.report(
                    self.scheduler_context.agent_name.as_deref(),
                    &request.decision_result_tx,
                );

                continue;
            }

            let Some(admission) = self
                .capacity_ledger
                .admit(request.layout.question_lane_demand())
            else {
                self.pending_requests.push_front(request);

                break;
            };

            if let Err(clear_error) = admission
                .state_sequence
                .sequence_id()
                .clear_kv_cache_in(&mut self.llama_context)
            {
                DecisionError::KvCacheRemovalFailed(clear_error).report(
                    self.scheduler_context.agent_name.as_deref(),
                    &request.decision_result_tx,
                );

                continue;
            }

            self.active_requests
                .push_back(ActiveDecisionRequest::new(request, admission));
        }
    }

    fn advance_next_active_request(&mut self) {
        let Some(mut request) = self.active_requests.pop_front() else {
            return;
        };
        let mut decoder = DecisionDecoder {
            batch: &mut self.batch,
            llama_context: &mut self.llama_context,
            n_batch: self.scheduler_context.n_batch,
        };
        let agent_name = self.scheduler_context.agent_name.as_deref();

        if request.is_stop_requested() {
            request.finish_with(&mut decoder, agent_name, DecisionResult::StopRequested);

            return;
        }

        match request.advance(
            &mut decoder,
            &self.scheduler_context.pointer_head,
            agent_name,
        ) {
            Ok(DecisionAdvance::Continuing) => self.active_requests.push_back(request),
            Ok(DecisionAdvance::Completed) => {
                let summary = request.summary();

                request.finish_with(&mut decoder, agent_name, summary);
            }
            Err(advance_error) => request.finish_with(&mut decoder, agent_name, advance_error),
        }
    }

    fn reject_unfinished_requests(&mut self) {
        let agent_name = self.scheduler_context.agent_name.as_deref();

        for request in self.active_requests.drain(..) {
            DecisionError::SchedulerUnavailable.report(agent_name, &request.decision_result_tx);
        }

        for request in self.pending_requests.drain(..) {
            DecisionError::SchedulerUnavailable.report(agent_name, &request.decision_result_tx);
        }

        self.scheduler_inbox.reject_waiting_commands(agent_name);
    }

    fn wait_for_next_scheduler_message(&mut self) {
        if let Some(request) = self.scheduler_inbox.wait_for_next_command() {
            self.pending_requests.push_back(request);
        }
    }
}
