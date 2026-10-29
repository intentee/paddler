#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_scheduler::DecisionScheduler;
use paddler_agent_decision::decision_scheduler_context::DecisionSchedulerContext;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_pointer_head::pointer_head::PointerHead;

use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_tests::kev_0_8b_pointer_head::kev_0_8b_pointer_head;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::two_option_question::two_option_question;

pub struct KevDecisionModel {
    pub delimiter_tokens: DecisionDelimiterTokens,
    pub pointer_head: PointerHead,
    pub runtime: LlamaRuntime,
}

impl KevDecisionModel {
    pub fn load(slots: u16) -> Result<Self> {
        LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(slots),
        )
        .context("the kev model must load")
        .and_then(|runtime| {
            kev_0_8b_pointer_head()
                .context("the kev pointer head must load")
                .and_then(|pointer_head| {
                    DecisionDelimiterTokens::resolve(
                        &runtime.loaded_llama_model.model.llama_model,
                        &pointer_head.delimiters,
                    )
                    .context("the kev pointer head delimiters must resolve")
                    .map(|delimiter_tokens| Self {
                        delimiter_tokens,
                        pointer_head,
                        runtime,
                    })
                })
        })
    }

    #[must_use]
    pub fn two_option_question_layout(&self, question_id: &str) -> DecisionTokenLayout {
        DecisionTokenLayout::new(
            &self.delimiter_tokens,
            vec![LlamaToken::new(0)],
            Vec::new(),
            two_option_question(question_id),
        )
    }

    pub fn run_scheduler(
        self,
        DecisionSchedulerRun {
            active_requests,
            agent_shutdown,
            capacity_ledger,
            pending_requests,
            scheduler_inbox,
            sequences,
        }: DecisionSchedulerRun,
    ) -> Result<()> {
        let Self {
            pointer_head,
            runtime,
            ..
        } = self;

        runtime
            .create_decision_llama_context(u32::from(sequences))
            .context("the decision context must be created")
            .map(|llama_context| {
                DecisionScheduler {
                    active_requests,
                    agent_shutdown,
                    batch: LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1),
                    capacity_ledger,
                    llama_context,
                    pending_requests,
                    scheduler_context: DecisionSchedulerContext {
                        agent_name: None,
                        n_batch: runtime.model_runtime_parameters.n_batch.tokens_usize(),
                        pointer_head,
                    },
                    scheduler_inbox,
                }
                .run();
            })
    }
}
