#![cfg(feature = "tests_that_use_llms")]

use libc::ENOMEM;
use libc::SYS_brk;
use libc::SYS_mmap;
use llama_cpp_bindings::error::SamplingError;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;

use paddler_agent_text_generation::conversation_support::ConversationSupport;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

const PENALTY_WINDOW_OF_FOUR_GIBIBYTES_OF_TOKENS: i32 = 1 << 30;
const SMALLEST_REFUSED_MAPPING_LENGTH: u64 = 1 << 32;
const ERRNO_THAT_MAKES_BRK_REPORT_A_NULL_BREAK: i32 = 0;

#[test]
fn a_sampler_chain_that_cannot_allocate_its_penalty_window_fails_the_generation_preparation() {
    let model_path = KevCheckpoint::Kev0_8b.target_path("model.gguf");
    let llama_runtime = LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
        .expect("the kev model must load");
    let sequence_context_size = llama_runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created")
        .n_ctx_seq();
    let model = &llama_runtime.loaded_llama_model.model;
    let request_preparer = GenerationRequestPreparer {
        conversation_support: ConversationSupport::ChatTemplateMissing(ModelPath {
            model_path: model_path.display().to_string(),
        }),
        image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
        loaded_llama_model: llama_runtime.loaded_llama_model.clone(),
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
        streaming_markers: model
            .streaming_markers()
            .expect("the model must report its streaming markers"),
    };
    let RawPromptGeneration { agent_request, .. } = RawPromptGeneration::new(
        "Paddler",
        llama_runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    RefusedSyscalls {
        errno: ENOMEM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_mmap,
            rules: vec![
                SeccompRule::new(vec![
                    SeccompCondition::new(
                        1,
                        SeccompCmpArgLen::Qword,
                        SeccompCmpOp::Ge,
                        SMALLEST_REFUSED_MAPPING_LENGTH,
                    )
                    .expect("the mapping length condition must be valid"),
                ])
                .expect("the mapping length rule must be valid"),
            ],
        }],
    }
    .apply_to_every_thread();

    RefusedSyscalls {
        errno: ERRNO_THAT_MAKES_BRK_REPORT_A_NULL_BREAK,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_brk,
            rules: vec![],
        }],
    }
    .apply_to_every_thread();

    assert_eq!(
        request_preparer
            .prepare_raw_prompt(agent_request)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TextGenerationError::SamplerChainCreationFailed(SamplingError::LlamaCppOutOfMemory {
                sampler: "penalties"
            })
            .to_string()
        )
    );
}
