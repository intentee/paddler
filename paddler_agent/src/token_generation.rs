use crate::generation_request_rejection::GenerationRequestRejection;
use crate::token_generation_support::TokenGenerationSupport;

pub enum TokenGeneration {
    DisabledForEmbeddings,
    Enabled(Box<TokenGenerationSupport>),
}

impl TokenGeneration {
    pub const fn require_enabled(
        &self,
    ) -> Result<&TokenGenerationSupport, GenerationRequestRejection> {
        match self {
            Self::DisabledForEmbeddings => Err(GenerationRequestRejection::TokenGenerationDisabled),
            Self::Enabled(token_generation_support) => Ok(token_generation_support),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::TokenGeneration;
    use crate::generation_request_rejection::GenerationRequestRejection;

    #[test]
    fn rejects_generation_in_embeddings_mode() {
        assert_eq!(
            TokenGeneration::DisabledForEmbeddings
                .require_enabled()
                .err()
                .map(|rejection| discriminant(&rejection)),
            Some(discriminant(
                &GenerationRequestRejection::TokenGenerationDisabled
            ))
        );
    }
}
