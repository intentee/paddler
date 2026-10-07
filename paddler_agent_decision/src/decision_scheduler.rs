use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvError;
use std::sync::mpsc::TryRecvError;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::info;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;

use crate::active_decision_request::ActiveDecisionRequest;
use crate::decision_advance::DecisionAdvance;
use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::prepared_decision_request::PreparedDecisionRequest;

pub struct DecisionScheduler<'model> {
    pub active_requests: VecDeque<ActiveDecisionRequest>,
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub capacity_ledger: DecisionCapacityLedger,
    pub llama_context: LlamaContext<'model>,
    pub pending_requests: VecDeque<PreparedDecisionRequest>,
    pub scheduler_context: DecisionSchedulerContext,
    pub scheduler_message_rx: Receiver<SchedulerMessage<PreparedDecisionRequest>>,
    pub shutdown_requested: bool,
}

impl DecisionScheduler<'_> {
    pub fn run(&mut self) {
        info!(
            "{:?}: decision scheduler started",
            self.scheduler_context.agent_name
        );

        while !self.agent_shutdown.is_cancelled() {
            if self.active_requests.is_empty() && self.pending_requests.is_empty() {
                if self.shutdown_requested {
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
        loop {
            match self.scheduler_message_rx.try_recv() {
                Ok(SchedulerMessage::Command(request)) => self.pending_requests.push_back(request),
                Ok(SchedulerMessage::Shutdown) | Err(TryRecvError::Disconnected) => {
                    self.shutdown_requested = true;

                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
    }

    fn admit_pending_requests(&mut self) {
        while let Some(mut request) = self.pending_requests.pop_front() {
            if request.decision_stop_rx.is_stop_requested() {
                continue;
            }

            let Some(admission) = self.capacity_ledger.admit(&request.layout) else {
                self.pending_requests.push_front(request);

                break;
            };

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
            if let Err(release_error) = request.release_sequences(&mut decoder) {
                release_error.report(agent_name, &request.decision_result_tx);
            }

            return;
        }

        match request.advance(&mut decoder, &self.scheduler_context.pointer_head) {
            Ok(DecisionAdvance::Continuing) => self.active_requests.push_back(request),
            Ok(DecisionAdvance::Completed) => match request.release_sequences(&mut decoder) {
                Ok(()) => {
                    send_result_or_warn(agent_name, &request.decision_result_tx, request.summary());
                }
                Err(release_error) => {
                    release_error.report(agent_name, &request.decision_result_tx);
                }
            },
            Err(advance_error) => {
                if let Err(release_error) = request.release_sequences(&mut decoder) {
                    release_error.report(agent_name, &request.decision_result_tx);
                }

                advance_error.report(agent_name, &request.decision_result_tx);
            }
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
    }

    fn wait_for_next_scheduler_message(&mut self) {
        match self.scheduler_message_rx.recv() {
            Ok(SchedulerMessage::Command(request)) => self.pending_requests.push_back(request),
            Ok(SchedulerMessage::Shutdown) | Err(RecvError) => self.shutdown_requested = true,
        }
    }
}
