use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_scheduler::DecisionScheduler;
use paddler_agent_decision::decision_scheduler_context::DecisionSchedulerContext;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_pointer_head::pointer_head::PointerHead;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::decision_scheduler_run::DecisionSchedulerRun;
use crate::kev_0_8b_pointer_head::kev_0_8b_pointer_head;
use crate::kev_checkpoint::KevCheckpoint;
use crate::llama_runtime::LlamaRuntime;
use crate::two_option_question::two_option_question;

pub struct KevDecisionModel {
    pub delimiter_tokens: DecisionDelimiterTokens,
    pub pointer_head: PointerHead,
    pub runtime: LlamaRuntime,
}

impl KevDecisionModel {
    /// # Errors
    ///
    /// Returns an error when the kev model, its pointer head, or the delimiters of that pointer head cannot load.
    pub fn load(slots: u16) -> Result<Self> {
        LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(slots),
        )
        .context("the kev model must load")
        .and_then(Self::with_the_kev_pointer_head)
    }

    fn with_the_kev_pointer_head(runtime: LlamaRuntime) -> Result<Self> {
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
    }

    /// # Errors
    ///
    /// Returns the [`DecisionError`] of [`LlamaRuntime::create_decision_llama_context`] when the decision context cannot be created.
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
    ) -> Result<(), DecisionError> {
        let Self {
            pointer_head,
            runtime,
            ..
        } = self;

        runtime
            .create_decision_llama_context(u32::from(sequences))
            .map(|llama_context| {
                DecisionScheduler {
                    active_requests,
                    agent_shutdown,
                    batch: LlamaBatch::new(ContextLimits::of(&llama_context)),
                    capacity_ledger,
                    llama_context,
                    pending_requests,
                    scheduler_context: DecisionSchedulerContext {
                        agent_name: None,
                        pointer_head,
                    },
                    scheduler_inbox,
                }
                .run();
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
}
