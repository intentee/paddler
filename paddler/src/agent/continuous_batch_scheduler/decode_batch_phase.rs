use llama_cpp_bindings::context::LlamaContext;

use crate::agent::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::agent::continuous_batch_scheduler::decode_outcome::DecodeOutcome;

pub fn run(pass: &mut BatchPass, context: &mut LlamaContext) -> DecodeOutcome {
    DecodeOutcome::from_decode_result(&context.decode(&mut pass.batch))
}
