use std::time::UNIX_EPOCH;

use llama_cpp_bindings_types::TokenUsage;
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
use paddler_openai_translation::generation_event::GenerationEvent;
use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;

#[cfg(test)]
fn first_response_snapshot_of(request: Value) -> Value {
    let sampling_parameters = SamplingParameters {
        temperature: 0.25,
        top_p: 0.5,
        ..SamplingParameters::default()
    };

    match from_value::<ResponsesRequest>(request)
        .expect("the request must deserialize")
        .translate(UNIX_EPOCH, &sampling_parameters)
        .expect("the request must translate")
        .delivery
    {
        ResponsesDelivery::Buffered(responses_response_header) => {
            to_value(responses_response_header.complete(
                "test-request",
                GeneratedOutput::default(),
                &GenerationSummary {
                    finish: GenerationFinish::EndOfGeneration,
                    usage: TokenUsage::new(),
                },
            ))
            .expect("a response must serialize")
        }
        ResponsesDelivery::Streamed(mut responses_stream) => to_value(
            &responses_stream.advance(GenerationEvent::Produced(GeneratedOutputPart::Content(
                "hello".to_owned(),
            )))[0],
        )
        .expect("an event must serialize")["response"]
            .clone(),
    }
}

#[test]
fn responses_requests_report_the_sampling_parameters_in_effect() {
    for (request, expected_status) in [
        (json!({"model": "test-model", "input": "hi"}), "completed"),
        (
            json!({"model": "test-model", "input": "hi", "stream": true}),
            "in_progress",
        ),
    ] {
        let response_snapshot = first_response_snapshot_of(request);

        assert_eq!(response_snapshot["status"], expected_status);
        assert!(
            response_snapshot["id"]
                .as_str()
                .is_some_and(|response_id| response_id.starts_with("resp_"))
        );
        assert_eq!(response_snapshot["temperature"], 0.25);
        assert_eq!(response_snapshot["top_p"], 0.5);
    }
}
