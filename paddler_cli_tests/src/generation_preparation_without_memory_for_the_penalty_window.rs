use anyhow::Context as _;
use anyhow::Result;
use libc::ENOMEM;
use libc::SYS_brk;
use libc::SYS_mmap;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::error::sampling_error::SamplingError;
use llama_cpp_bindings::model::llama_chat_template::LlamaChatTemplate;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;

use paddler_agent_text_generation::conversation_prompt_support::ConversationPromptSupport;
use paddler_agent_text_generation::conversation_support::ConversationSupport;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

const PENALTY_WINDOW_OF_FOUR_GIBIBYTES_OF_TOKENS: i32 = 1 << 30;
const SMALLEST_REFUSED_MAPPING_LENGTH: u64 = 1 << 32;
const ERRNO_THAT_MAKES_BRK_REPORT_A_NULL_BREAK: i32 = 0;

fn request_preparer_for(
    LlamaRuntime {
        loaded_llama_model, ..
    }: &LlamaRuntime,
    sequence_context_size: u32,
) -> Result<GenerationRequestPreparer> {
    let model = &loaded_llama_model.model;

    LlamaChatTemplate::new(&first_message_content_chat_template().content)
        .context("the first message content chat template holds a NUL")
        .and_then(|chat_template| {
            ChatTemplates::with_override(model, &chat_template)
                .context("llama.cpp cannot prepare the first message content chat template")
        })
        .map(|chat_templates| GenerationRequestPreparer {
            conversation_support: ConversationSupport::Available(Box::new(
                ConversationPromptSupport {
                    chat_templates,
                    image_input: ImageInput::Unsupported,
                },
            )),
            image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
            loaded_llama_model: loaded_llama_model.clone(),
            prompt_tokenizer: PromptTokenizer {
                model: model.clone(),
                model_bos_token: ModelBosToken::of(model),
                sequence_context_size,
            },
            sampler_chain_factory: SamplerChainFactory {
                sampling_parameters: SamplingParameters {
                    penalty_last_n: PENALTY_WINDOW_OF_FOUR_GIBIBYTES_OF_TOKENS,
                    penalty_repeat: 1.1,
                    ..SamplingParameters::default()
                },
                n_vocab: model.n_vocab(),
            },
        })
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
    /// Fails when the kev model cannot load, when its text generation context cannot be created, and when llama.cpp cannot prepare the first message content chat template.
    pub fn load() -> Result<Self> {
        LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(1),
        )
        .context("the kev model cannot load")
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

    #[must_use]
    pub fn penalty_window_allocation_failure() -> String {
        TextGenerationError::SamplerChainCreationFailed(SamplingError::LlamaCppOutOfMemory {
            sampler: "penalties",
        })
        .to_string()
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
