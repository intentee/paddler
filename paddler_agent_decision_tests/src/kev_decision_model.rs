use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_model::llama_token::LlamaToken;

use paddler_agent_decision::constants::KEV_DECISION_DELIMITERS;
use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_scheduler::DecisionScheduler;
use paddler_agent_decision::decision_scheduler_context::DecisionSchedulerContext;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_decision::kev_option_scorer::KevOptionScorer;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

use crate::decision_scheduler_run::DecisionSchedulerRun;
use crate::two_option_question::two_option_question;

pub struct KevDecisionModel {
    pub delimiter_tokens: DecisionDelimiterTokens,
    pub option_scorer: KevOptionScorer,
    pub runtime: LlamaRuntime,
}

impl KevDecisionModel {
    /// # Errors
    ///
    /// Returns an error when the kev model is not cached or cannot load, when it cannot score options, and when its vocabulary does not hold the kev delimiters.
    pub fn load(slots: u16) -> Result<Self> {
        kev_0_8b()
            .cached_path()
            .context("the kev model must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(slots))
                    .context("the kev model must load")
            })
            .and_then(Self::scoring_with_the_kev_delimiters)
    }

    fn scoring_with_the_kev_delimiters(runtime: LlamaRuntime) -> Result<Self> {
        let loaded_llama_model = &runtime.loaded_llama_model;
        let option_scorer = KevOptionScorer::of(
            &loaded_llama_model.model_metadata,
            loaded_llama_model.model.n_embd_out(),
        )
        .context("the kev model must score options")?;
        let delimiter_tokens = DecisionDelimiterTokens::resolve(
            &*loaded_llama_model.model.llama_model,
            &KEV_DECISION_DELIMITERS,
        )
        .context("the kev delimiters must resolve")?;

        Ok(Self {
            delimiter_tokens,
            option_scorer,
            runtime,
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
            option_scorer,
            runtime,
            ..
        } = self;

        runtime
            .create_decision_llama_context(u32::from(sequences))
            .map(|llama_context| {
                DecisionScheduler {
                    active_requests,
                    agent_shutdown,
                    batch: LlamaBatch::new(llama_context.limits()),
                    capacity_ledger,
                    llama_context,
                    pending_requests,
                    scheduler_context: DecisionSchedulerContext {
                        agent_name: None,
                        option_scorer,
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
