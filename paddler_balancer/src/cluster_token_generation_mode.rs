use actix_web::Error;
use actix_web::error::ErrorNotImplemented;

pub const TOKEN_GENERATION_DISABLED_MESSAGE: &str =
    "Token generation is disabled while the cluster is configured for embeddings";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClusterTokenGenerationMode {
    Enabled,
    DisabledForEmbeddings,
}

impl ClusterTokenGenerationMode {
    pub fn require_enabled(self) -> Result<(), Error> {
        match self {
            Self::Enabled => Ok(()),
            Self::DisabledForEmbeddings => {
                Err(ErrorNotImplemented(TOKEN_GENERATION_DISABLED_MESSAGE))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;

    use super::ClusterTokenGenerationMode;

    #[test]
    fn allows_requests_while_token_generation_is_enabled() {
        assert!(
            ClusterTokenGenerationMode::Enabled
                .require_enabled()
                .is_ok()
        );
    }

    #[test]
    fn rejects_requests_as_not_implemented_while_disabled_for_embeddings() {
        let rejection = ClusterTokenGenerationMode::DisabledForEmbeddings
            .require_enabled()
            .expect_err("token generation must be rejected in embeddings mode");

        assert_eq!(
            rejection.as_response_error().status_code(),
            StatusCode::NOT_IMPLEMENTED
        );
    }
}
