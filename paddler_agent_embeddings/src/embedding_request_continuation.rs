use std::option::IntoIter;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;

use crate::active_embedding_request::ActiveEmbeddingRequest;

pub enum EmbeddingRequestContinuation {
    Concluded,
    Continues(ActiveEmbeddingRequest),
}

impl EmbeddingRequestContinuation {
    #[must_use]
    pub fn unless_every_input_is_embedded(
        active_request: ActiveEmbeddingRequest,
        agent_name: Option<&str>,
    ) -> Self {
        if active_request.pending_inputs.is_empty() {
            active_request.finish(agent_name);

            return Self::Concluded;
        }

        Self::Continues(active_request)
    }

    #[must_use]
    pub fn unless_stop_requested(
        mut active_request: ActiveEmbeddingRequest,
        agent_name: Option<&str>,
    ) -> Self {
        if active_request
            .generate_embedding_stop_rx
            .is_stop_requested()
        {
            active_request.finish(agent_name);

            return Self::Concluded;
        }

        Self::Continues(active_request)
    }
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
