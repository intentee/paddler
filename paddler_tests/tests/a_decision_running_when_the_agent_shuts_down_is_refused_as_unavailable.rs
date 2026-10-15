#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::sync::mpsc::channel;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_scheduler::DecisionScheduler;
use paddler_agent_decision::decision_scheduler_context::DecisionSchedulerContext;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_pointer_head::kev_0_8b_pointer_head;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::two_option_question::two_option_question;

#[test]
fn a_decision_running_when_the_agent_shuts_down_is_refused_as_unavailable() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev decision model must load");
    let pointer_head = kev_0_8b_pointer_head().expect("the kev pointer head must load");
    let delimiter_tokens = DecisionDelimiterTokens::resolve(
        runtime.loaded_llama_model.model.as_ref(),
        &pointer_head.delimiters,
    )
    .expect("the kev delimiters must resolve");
    let llama_context = runtime
        .create_decision_llama_context(2)
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let capacity_ledger = DecisionCapacityLedger::new(llama_context.n_ctx() as usize, 2);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0)],
        Vec::new(),
        two_option_question("question"),
    );
    let admission = capacity_ledger
        .admit(&layout.capacity_demand())
        .expect("the decision must be admitted");
    let AdmittedDecision {
        active_decision_request,
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::new(
        admission,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    );
    let agent_shutdown = CancellationToken::new();
    let (_scheduler_message_tx, scheduler_message_rx) = channel();

    agent_shutdown.cancel();

    DecisionScheduler {
        active_requests: VecDeque::from([active_decision_request]),
        agent_shutdown,
        batch: LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1),
        capacity_ledger,
        llama_context,
        pending_requests: VecDeque::new(),
        scheduler_context: DecisionSchedulerContext {
            agent_name: None,
            n_batch,
            pointer_head,
        },
        scheduler_message_rx,
        shutdown_requested: false,
    }
    .run();

    assert_eq!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::SchedulerUnavailable(
            DecisionError::SchedulerUnavailable.to_string()
        ))
    );
}
