use llama_cpp_bindings::MarkerRole;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::StreamingMarkers;
use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;

use crate::model_constants_error::ModelConstantsError;

fn decode_special_token(
    model: &LlamaModel,
    decoder: &mut encoding_rs::Decoder,
    token: LlamaToken,
) -> Result<String, TokenToStringError> {
    model.token_to_piece(&SampledToken::Content(token), decoder, true, None)
}

fn closes_reasoning(streaming_markers: &StreamingMarkers) -> bool {
    streaming_markers
        .iter()
        .any(|marker| marker.roles().contains(&MarkerRole::ReasoningClose))
}

pub struct ModelConstants {
    pub closes_reasoning: bool,
    pub n_vocab: i32,
    pub streaming_markers: StreamingMarkers,
    pub token_bos_str: String,
    pub token_eos_str: String,
    pub token_nl_str: String,
}

impl ModelConstants {
    pub fn from_model(model: &LlamaModel) -> Result<Self, ModelConstantsError> {
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let streaming_markers = model
            .streaming_markers()
            .map_err(ModelConstantsError::StreamingMarkersNotDetectable)?;

        Ok(Self {
            closes_reasoning: closes_reasoning(&streaming_markers),
            n_vocab: model.n_vocab(),
            streaming_markers,
            token_bos_str: decode_special_token(model, &mut decoder, model.token_bos())
                .map_err(ModelConstantsError::BosTokenNotDecodable)?,
            token_eos_str: decode_special_token(model, &mut decoder, model.token_eos())
                .map_err(ModelConstantsError::EosTokenNotDecodable)?,
            token_nl_str: decode_special_token(model, &mut decoder, model.token_nl())
                .map_err(ModelConstantsError::NewlineTokenNotDecodable)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::MarkerRole;
    use llama_cpp_bindings::MarkerRoleCandidate;
    use llama_cpp_bindings::StreamingMarkers;
    use llama_cpp_bindings::token::LlamaToken;

    use super::ModelConstants;

    fn constants_with_roles(roles: Vec<MarkerRole>) -> ModelConstants {
        let candidates = roles
            .into_iter()
            .enumerate()
            .map(|(index, role)| MarkerRoleCandidate {
                tokens: vec![LlamaToken::new(i32::try_from(index).unwrap() + 1)],
                role,
            })
            .collect::<Vec<_>>();

        let streaming_markers = StreamingMarkers::from_candidates(candidates).unwrap();

        ModelConstants {
            closes_reasoning: super::closes_reasoning(&streaming_markers),
            n_vocab: 32,
            streaming_markers,
            token_bos_str: String::new(),
            token_eos_str: String::new(),
            token_nl_str: String::new(),
        }
    }

    #[test]
    fn a_model_exposing_a_reasoning_close_marker_closes_reasoning() {
        let constants =
            constants_with_roles(vec![MarkerRole::ReasoningOpen, MarkerRole::ReasoningClose]);

        assert!(constants.closes_reasoning);
    }

    #[test]
    fn a_model_whose_markers_never_close_reasoning_does_not_close_reasoning() {
        let constants =
            constants_with_roles(vec![MarkerRole::ToolCallOpen, MarkerRole::ToolCallClose]);

        assert!(!constants.closes_reasoning);
    }

    #[test]
    fn a_model_without_any_markers_does_not_close_reasoning() {
        let constants = constants_with_roles(Vec::new());

        assert!(!constants.closes_reasoning);
    }
}
