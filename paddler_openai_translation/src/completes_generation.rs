use serde::Serialize;

use paddler_messaging::generation_summary::GenerationSummary;

use crate::generated_output::GeneratedOutput;

pub trait CompletesGeneration {
    type Completion: Serialize;

    fn complete(
        &self,
        request_id: &str,
        generated_output: GeneratedOutput,
        generation_summary: &GenerationSummary,
    ) -> Self::Completion;
}
