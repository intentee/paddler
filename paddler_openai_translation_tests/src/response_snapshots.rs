use std::time::UNIX_EPOCH;

use llama_cpp_bindings_types::token_usage::TokenUsage;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_value;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_openai_translation::completes_generation::CompletesGeneration as _;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generated_output_part::GeneratedOutputPart;
use paddler_openai_translation::generation_progress::GenerationProgress;
use paddler_openai_translation::response_snapshot_event::ResponseSnapshotEvent;
use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;
use paddler_openai_translation::responses_stream_event::ResponsesStreamEvent;
use paddler_openai_translation::sequenced_responses_stream_event::SequencedResponsesStreamEvent;
use paddler_openai_translation::translated_responses_request::TranslatedResponsesRequest;

use crate::openai_translation_tests_error::OpenAITranslationTestsError;

fn response_snapshots_of_delivery(
    delivery: ResponsesDelivery,
) -> Result<Vec<Value>, OpenAITranslationTestsError> {
    match delivery {
        ResponsesDelivery::Buffered(responses_response_header) => {
            to_value(responses_response_header.complete(
                "test-request",
                GeneratedOutput::default(),
                &GenerationSummary {
                    finish: GenerationFinish::EndOfGeneration,
                    usage: TokenUsage::new(),
                },
            ))
            .map(|completed_response| vec![completed_response])
            .map_err(OpenAITranslationTestsError::ResponseUnserializable)
        }
        ResponsesDelivery::Streamed(mut responses_stream) => responses_stream
            .advance(GenerationProgress::Produced(GeneratedOutputPart::Content(
                "hello".to_owned(),
            )))
            .iter()
            .filter_map(|SequencedResponsesStreamEvent { event, .. }| match event {
                ResponsesStreamEvent::Created(ResponseSnapshotEvent { response })
                | ResponsesStreamEvent::InProgress(ResponseSnapshotEvent { response }) => {
                    Some(to_value(response))
                }
                _ => None,
            })
            .collect::<Result<Vec<Value>, _>>()
            .map_err(OpenAITranslationTestsError::EventUnserializable),
    }
}

fn response_snapshots_of(request: Value) -> Result<Vec<Value>, OpenAITranslationTestsError> {
    let sampling_parameters = SamplingParameters {
        temperature: 0.25,
        top_p: 0.5,
        ..SamplingParameters::default()
    };

    from_value::<ResponsesRequest>(request)
        .map_err(OpenAITranslationTestsError::RequestUndeserializable)
        .and_then(|responses_request| {
            responses_request
                .translate(UNIX_EPOCH, &sampling_parameters)
                .map_err(OpenAITranslationTestsError::RequestUntranslatable)
        })
        .and_then(|TranslatedResponsesRequest { delivery, .. }| {
            response_snapshots_of_delivery(delivery)
        })
}

pub struct ResponseSnapshots {
    pub buffered: Vec<Value>,
    pub streamed: Vec<Value>,
}

impl ResponseSnapshots {
    /// # Errors
    ///
    /// Returns [`OpenAITranslationTestsError::RequestUndeserializable`] when a request is not a responses request, [`OpenAITranslationTestsError::RequestUntranslatable`] when a request cannot be translated, and [`OpenAITranslationTestsError::ResponseUnserializable`] or [`OpenAITranslationTestsError::EventUnserializable`] when the buffered response or a response the stream opens with cannot be serialized.
    pub fn of_buffered_and_streamed_requests() -> Result<Self, OpenAITranslationTestsError> {
        response_snapshots_of(json!({"model": "test-model", "input": "hi"})).and_then(|buffered| {
            response_snapshots_of(json!({"model": "test-model", "input": "hi", "stream": true}))
                .map(|streamed| Self { buffered, streamed })
        })
    }
}
