use crate::generated_token_final_result::GeneratedTokenFinalResult;
use crate::generated_token_partial_result::GeneratedTokenPartialResult;

#[derive(Debug, Eq, PartialEq)]
pub enum ResultStep<TPartial, TFinal> {
    Final(TFinal),
    Partial(TPartial),
}

impl From<GeneratedTokenFinalResult>
    for ResultStep<GeneratedTokenPartialResult, GeneratedTokenFinalResult>
{
    fn from(final_result: GeneratedTokenFinalResult) -> Self {
        Self::Final(final_result)
    }
}

impl From<GeneratedTokenPartialResult>
    for ResultStep<GeneratedTokenPartialResult, GeneratedTokenFinalResult>
{
    fn from(partial_result: GeneratedTokenPartialResult) -> Self {
        Self::Partial(partial_result)
    }
}
