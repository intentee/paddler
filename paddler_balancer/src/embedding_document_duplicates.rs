use std::collections::HashMap;
use std::iter::once;

use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use paddler_messaging::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct EmbeddingDocumentDuplicates {
    pub duplicate_document_ids: HashMap<String, Vec<String>>,
}

impl EmbeddingDocumentDuplicates {
    #[must_use]
    pub fn fan_out(&self, message: OutgoingMessage) -> Vec<OutgoingMessage> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                generated_by,
                request_id,
                response: OutgoingResponse::Embedding(embedding_result),
            }) => {
                let duplicate_results = self.results_for_duplicates_of(&embedding_result);

                once(embedding_result)
                    .chain(duplicate_results)
                    .map(|fanned_out_result| {
                        OutgoingMessage::Response(ResponseEnvelope {
                            generated_by: generated_by.clone(),
                            request_id: request_id.clone(),
                            response: OutgoingResponse::Embedding(fanned_out_result),
                        })
                    })
                    .collect()
            }
            other_message => vec![other_message],
        }
    }

    fn duplicates_of<'duplicates>(
        &'duplicates self,
        source_document_id: &str,
    ) -> impl Iterator<Item = &'duplicates String> {
        self.duplicate_document_ids
            .get(source_document_id)
            .into_iter()
            .flatten()
    }

    fn results_for_duplicates_of(
        &self,
        embedding_result: &EmbeddingResult,
    ) -> Vec<EmbeddingResult> {
        match embedding_result {
            EmbeddingResult::DocumentExceedsBatchSize(oversized_document) => self
                .duplicates_of(&oversized_document.source_document_id)
                .map(|duplicate_document_id| {
                    EmbeddingResult::DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails {
                        source_document_id: duplicate_document_id.clone(),
                        ..oversized_document.clone()
                    })
                })
                .collect(),
            EmbeddingResult::DocumentExceedsSequenceContext(oversized_document) => self
                .duplicates_of(&oversized_document.source_document_id)
                .map(|duplicate_document_id| {
                    EmbeddingResult::DocumentExceedsSequenceContext(
                        EmbeddingDocumentExceedingSequenceContextDetails {
                            source_document_id: duplicate_document_id.clone(),
                            ..oversized_document.clone()
                        },
                    )
                })
                .collect(),
            EmbeddingResult::DocumentTokenizedToNothing(empty_document) => self
                .duplicates_of(&empty_document.source_document_id)
                .map(|duplicate_document_id| {
                    EmbeddingResult::DocumentTokenizedToNothing(
                        EmbeddingDocumentTokenizedToNothingDetails {
                            source_document_id: duplicate_document_id.clone(),
                        },
                    )
                })
                .collect(),
            EmbeddingResult::Embedding(embedding) => self
                .duplicates_of(&embedding.source_document_id)
                .map(|duplicate_document_id| {
                    EmbeddingResult::Embedding(Embedding {
                        source_document_id: duplicate_document_id.clone(),
                        ..embedding.clone()
                    })
                })
                .collect(),
            EmbeddingResult::AgentRuntimeFailed(_)
            | EmbeddingResult::BatchAssemblyFailed(_)
            | EmbeddingResult::DecodeFailed(_)
            | EmbeddingResult::Done
            | EmbeddingResult::EmbeddingTooLongForRmsNormalization(_)
            | EmbeddingResult::EmbeddingsUnavailable(_)
            | EmbeddingResult::InputTokenizationFailed(_)
            | EmbeddingResult::ModelNotLoaded(_)
            | EmbeddingResult::NoEmbeddingsProduced
            | EmbeddingResult::SchedulerUnavailable(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::to_string;

    use paddler_inference_parameters::pooling_type::PoolingType;
    use paddler_messaging::embedding::Embedding;
    use paddler_messaging::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
    use paddler_messaging::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::embedding_vector::EmbeddingVector;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

    use super::EmbeddingDocumentDuplicates;

    fn duplicates_of_first_document() -> EmbeddingDocumentDuplicates {
        EmbeddingDocumentDuplicates {
            duplicate_document_ids: HashMap::from([(
                "first".to_owned(),
                vec!["second".to_owned(), "third".to_owned()],
            )]),
        }
    }

    fn embedding_response(embedding_result: EmbeddingResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: Some("agent".to_owned()),
            request_id: "request".to_owned(),
            response: OutgoingResponse::Embedding(embedding_result),
        })
    }

    fn embedding_of(source_document_id: &str) -> EmbeddingResult {
        EmbeddingResult::Embedding(Embedding {
            embedding: EmbeddingVector::from(vec![0.25, 0.75]),
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Mean,
            source_document_id: source_document_id.to_owned(),
        })
    }

    fn oversized_document(source_document_id: &str) -> EmbeddingResult {
        EmbeddingResult::DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails {
            document_tokens: 900,
            n_batch: 512,
            source_document_id: source_document_id.to_owned(),
        })
    }

    fn document_exceeding_sequence_context(source_document_id: &str) -> EmbeddingResult {
        EmbeddingResult::DocumentExceedsSequenceContext(
            EmbeddingDocumentExceedingSequenceContextDetails {
                document_tokens: 300,
                sequence_context_size: 256,
                source_document_id: source_document_id.to_owned(),
            },
        )
    }

    fn document_tokenized_to_nothing(source_document_id: &str) -> EmbeddingResult {
        EmbeddingResult::DocumentTokenizedToNothing(EmbeddingDocumentTokenizedToNothingDetails {
            source_document_id: source_document_id.to_owned(),
        })
    }

    fn serialized(messages: &[OutgoingMessage]) -> Vec<String> {
        messages
            .iter()
            .map(|message| to_string(message).unwrap())
            .collect()
    }

    #[test]
    fn fans_an_embedding_out_to_every_document_sharing_its_content() {
        assert_eq!(
            serialized(
                &duplicates_of_first_document().fan_out(embedding_response(embedding_of("first")))
            ),
            serialized(&[
                embedding_response(embedding_of("first")),
                embedding_response(embedding_of("second")),
                embedding_response(embedding_of("third")),
            ])
        );
    }

    #[test]
    fn fans_an_oversized_document_out_to_every_document_sharing_its_content() {
        assert_eq!(
            serialized(
                &duplicates_of_first_document()
                    .fan_out(embedding_response(oversized_document("first")))
            ),
            serialized(&[
                embedding_response(oversized_document("first")),
                embedding_response(oversized_document("second")),
                embedding_response(oversized_document("third")),
            ])
        );
    }

    #[test]
    fn fans_a_document_exceeding_its_sequence_context_out_to_every_document_sharing_its_content() {
        assert_eq!(
            serialized(&duplicates_of_first_document().fan_out(embedding_response(
                document_exceeding_sequence_context("first")
            ))),
            serialized(&[
                embedding_response(document_exceeding_sequence_context("first")),
                embedding_response(document_exceeding_sequence_context("second")),
                embedding_response(document_exceeding_sequence_context("third")),
            ])
        );
    }

    #[test]
    fn fans_a_document_tokenized_to_nothing_out_to_every_document_sharing_its_content() {
        assert_eq!(
            serialized(
                &duplicates_of_first_document()
                    .fan_out(embedding_response(document_tokenized_to_nothing("first")))
            ),
            serialized(&[
                embedding_response(document_tokenized_to_nothing("first")),
                embedding_response(document_tokenized_to_nothing("second")),
                embedding_response(document_tokenized_to_nothing("third")),
            ])
        );
    }

    #[test]
    fn passes_a_result_of_a_whole_chunk_through_once() {
        assert_eq!(
            serialized(
                &duplicates_of_first_document()
                    .fan_out(embedding_response(EmbeddingResult::NoEmbeddingsProduced))
            ),
            serialized(&[embedding_response(EmbeddingResult::NoEmbeddingsProduced)])
        );
    }

    #[test]
    fn passes_an_error_through_once() {
        let error = || {
            OutgoingMessage::Error(ErrorEnvelope {
                request_id: "request".to_owned(),
                error: JsonRpcError {
                    code: 504,
                    description: "timed out".to_owned(),
                },
            })
        };

        assert_eq!(
            serialized(&duplicates_of_first_document().fan_out(error())),
            serialized(&[error()])
        );
    }
}
