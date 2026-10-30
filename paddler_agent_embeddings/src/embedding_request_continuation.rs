use std::option::IntoIter;

use crate::active_embedding_request::ActiveEmbeddingRequest;

pub enum EmbeddingRequestContinuation {
    Concluded,
    Continues(ActiveEmbeddingRequest),
}

impl IntoIterator for EmbeddingRequestContinuation {
    type IntoIter = IntoIter<ActiveEmbeddingRequest>;
    type Item = ActiveEmbeddingRequest;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Concluded => None.into_iter(),
            Self::Continues(active_request) => Some(active_request).into_iter(),
        }
    }
}
