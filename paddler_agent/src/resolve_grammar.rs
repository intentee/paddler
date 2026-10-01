use llama_cpp_bindings::json_schema_to_grammar;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::grammar_sampling::GrammarSampling;

const JSON_SCHEMA_GRAMMAR_ROOT: &str = "root";

pub fn resolve_grammar(
    grammar: Option<GrammarConstraint>,
    model: &LlamaModel,
) -> Result<GrammarSampling, GenerationRequestRejection> {
    match grammar {
        None => Ok(GrammarSampling::Unconstrained),
        Some(GrammarConstraint::Gbnf { grammar, root }) => {
            LlamaSampler::grammar(model, &grammar, &root)
                .map(GrammarSampling::Constrained)
                .map_err(GenerationRequestRejection::for_gbnf_grammar_error)
        }
        Some(GrammarConstraint::JsonSchema { schema }) => {
            let grammar = json_schema_to_grammar(&schema)
                .map_err(GenerationRequestRejection::GrammarConversionFailed)?;

            LlamaSampler::grammar(model, &grammar, JSON_SCHEMA_GRAMMAR_ROOT)
                .map(GrammarSampling::Constrained)
                .map_err(GenerationRequestRejection::GrammarSamplerInitializationFailed)
        }
    }
}
