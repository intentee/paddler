use anyhow::Context as _;
use anyhow::Result;
use libc::ENOMEM;
use libc::SYS_brk;
use libc::SYS_mmap;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_refused_syscalls::refused_syscall::RefusedSyscall;
use paddler_refused_syscalls::refused_syscalls::RefusedSyscalls;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

const PENALTY_WINDOW_OF_FOUR_GIBIBYTES_OF_TOKENS: i32 = 1 << 30;
const SMALLEST_REFUSED_MAPPING_LENGTH: u64 = 1 << 32;
const ERRNO_THAT_MAKES_BRK_REPORT_A_NULL_BREAK: i32 = 0;

fn request_preparer_for(
    llama_runtime: &LlamaRuntime,
    sequence_context_size: u32,
) -> Result<GenerationRequestPreparer> {
    FirstMessageContentRequestPreparer {
        llama_runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer()
    .map(
        |first_message_content_request_preparer| GenerationRequestPreparer {
            sampler_chain_factory: SamplerChainFactory {
                sampling_parameters: SamplingParameters {
                    penalty_last_n: PENALTY_WINDOW_OF_FOUR_GIBIBYTES_OF_TOKENS,
                    penalty_repeat: 1.1,
                    ..SamplingParameters::deterministic()
                },
                n_vocab: llama_runtime.loaded_llama_model.model.n_vocab(),
            },
            ..first_message_content_request_preparer
        },
    )
}

fn mapping_length_rule() -> Result<SeccompRule> {
    SeccompCondition::new(
        1,
        SeccompCmpArgLen::Qword,
        SeccompCmpOp::Ge,
        SMALLEST_REFUSED_MAPPING_LENGTH,
    )
    .context("the mapping length condition is invalid")
    .and_then(|mapping_length_condition| {
        SeccompRule::new(vec![mapping_length_condition])
            .context("the mapping length rule is invalid")
    })
}

pub struct GenerationPreparationWithoutMemoryForThePenaltyWindow {
    pub llama_runtime: LlamaRuntime,
    pub request_preparer: GenerationRequestPreparer,
}

impl GenerationPreparationWithoutMemoryForThePenaltyWindow {
    /// # Errors
    ///
    /// Fails when the model cannot load, when its text generation context cannot be created, when llama.cpp cannot prepare the first message content chat template, and when the capabilities of the model cannot be established.
    pub fn load() -> Result<Self> {
        smolvlm2_256m()
            .cached_path()
            .context("the model is not in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                    .context("the model cannot load")
            })
            .and_then(Self::preparing_generation_on)
    }

    fn preparing_generation_on(llama_runtime: LlamaRuntime) -> Result<Self> {
        llama_runtime
            .create_text_generation_llama_context(1)
            .map(|text_generation_context| text_generation_context.n_ctx_seq())
            .context("the text generation context cannot be created")
            .and_then(|sequence_context_size| {
                request_preparer_for(&llama_runtime, sequence_context_size)
            })
            .map(|request_preparer| Self {
                llama_runtime,
                request_preparer,
            })
    }

    /// # Errors
    ///
    /// Fails when the rules refusing the penalty window allocation are invalid, or cannot apply to every thread of the test process.
    pub fn refuse_the_penalty_window_allocation() -> Result<()> {
        mapping_length_rule()
            .and_then(|mapping_length_rule| {
                RefusedSyscalls {
                    errno: ENOMEM,
                    refused_syscalls: vec![RefusedSyscall {
                        syscall: SYS_mmap,
                        rules: vec![mapping_length_rule],
                    }],
                }
                .apply_to_every_thread()
            })
            .and_then(|()| {
                RefusedSyscalls {
                    errno: ERRNO_THAT_MAKES_BRK_REPORT_A_NULL_BREAK,
                    refused_syscalls: vec![RefusedSyscall {
                        syscall: SYS_brk,
                        rules: vec![],
                    }],
                }
                .apply_to_every_thread()
            })
    }
}
