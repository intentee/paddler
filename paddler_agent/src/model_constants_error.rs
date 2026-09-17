use llama_cpp_bindings::error::MarkerDetectionError;
use llama_cpp_bindings::error::TokenToStringError;

#[derive(Debug, thiserror::Error)]
pub enum ModelConstantsError {
    #[error("the beginning-of-sequence token could not be decoded: {0}")]
    BosTokenNotDecodable(#[source] TokenToStringError),
    #[error("the end-of-sequence token could not be decoded: {0}")]
    EosTokenNotDecodable(#[source] TokenToStringError),
    #[error("the newline token could not be decoded: {0}")]
    NewlineTokenNotDecodable(#[source] TokenToStringError),
    #[error("the model's streaming markers could not be detected: {0}")]
    StreamingMarkersNotDetectable(#[source] MarkerDetectionError),
}
