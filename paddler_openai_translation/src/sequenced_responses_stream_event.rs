use serde::Serialize;
use serde::Serializer;

use crate::responses_stream_event::ResponsesStreamEvent;

#[derive(Serialize)]
struct WithEmptyLogprobs<'event, TEvent> {
    #[serde(flatten)]
    event: &'event TEvent,
    logprobs: &'static [u8],
}

#[derive(Serialize)]
struct TypedEvent<'event, TPayload> {
    #[serde(rename = "type")]
    event_type: &'static str,
    sequence_number: u64,
    #[serde(flatten)]
    payload: &'event TPayload,
}

#[derive(Clone, Debug)]
pub struct SequencedResponsesStreamEvent {
    pub event: ResponsesStreamEvent,
    pub sequence_number: u64,
}

impl SequencedResponsesStreamEvent {
    fn serialize_payload<TPayload, TSerializer>(
        &self,
        payload: &TPayload,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TPayload: Serialize,
        TSerializer: Serializer,
    {
        TypedEvent {
            event_type: self.event.event_name(),
            sequence_number: self.sequence_number,
            payload,
        }
        .serialize(serializer)
    }
}

impl Serialize for SequencedResponsesStreamEvent {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        match &self.event {
            ResponsesStreamEvent::Created(snapshot)
            | ResponsesStreamEvent::InProgress(snapshot)
            | ResponsesStreamEvent::Completed(snapshot)
            | ResponsesStreamEvent::Incomplete(snapshot)
            | ResponsesStreamEvent::Failed(snapshot) => {
                self.serialize_payload(snapshot, serializer)
            }
            ResponsesStreamEvent::OutputItemAdded(item_event)
            | ResponsesStreamEvent::OutputItemDone(item_event) => {
                self.serialize_payload(item_event, serializer)
            }
            ResponsesStreamEvent::ContentPartAdded(part_event)
            | ResponsesStreamEvent::ContentPartDone(part_event) => {
                self.serialize_payload(part_event, serializer)
            }
            ResponsesStreamEvent::OutputTextDelta(delta_event) => self.serialize_payload(
                &WithEmptyLogprobs {
                    event: delta_event,
                    logprobs: &[],
                },
                serializer,
            ),
            ResponsesStreamEvent::ReasoningTextDelta(delta_event) => {
                self.serialize_payload(delta_event, serializer)
            }
            ResponsesStreamEvent::OutputTextDone(done_event) => self.serialize_payload(
                &WithEmptyLogprobs {
                    event: done_event,
                    logprobs: &[],
                },
                serializer,
            ),
            ResponsesStreamEvent::ReasoningTextDone(done_event) => {
                self.serialize_payload(done_event, serializer)
            }
            ResponsesStreamEvent::FunctionCallArgumentsDelta(arguments_event) => {
                self.serialize_payload(arguments_event, serializer)
            }
            ResponsesStreamEvent::FunctionCallArgumentsDone(arguments_event) => {
                self.serialize_payload(arguments_event, serializer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use serde_json::to_value;

    use super::SequencedResponsesStreamEvent;
    use crate::response_snapshot_event::ResponseSnapshotEvent;
    use crate::responses_response_header::ResponsesResponseHeader;
    use crate::responses_stream_event::ResponsesStreamEvent;
    use crate::text_delta_event::TextDeltaEvent;

    #[test]
    fn serialized_type_field_matches_event_name() {
        let sequenced_event = SequencedResponsesStreamEvent {
            event: ResponsesStreamEvent::Completed(ResponseSnapshotEvent {
                response: ResponsesResponseHeader {
                    created_at: 0,
                    id: "resp_0".to_owned(),
                    instructions: None,
                    model: "test-model".to_owned(),
                    temperature: 1.0,
                    top_p: 1.0,
                }
                .in_progress(),
            }),
            sequence_number: 7,
        };

        let serialized =
            to_value(&sequenced_event).expect("a responses stream event must serialize");

        assert_eq!(
            serialized.pointer("/type"),
            Some(&json!(sequenced_event.event.event_name()))
        );
        assert_eq!(serialized.pointer("/sequence_number"), Some(&json!(7)));
        assert_eq!(serialized.pointer("/response/id"), Some(&json!("resp_0")));
    }

    #[test]
    fn text_delta_includes_the_required_logprobs_array() {
        let sequenced_event = SequencedResponsesStreamEvent {
            event: ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
                item_id: "msg_0".to_owned(),
                output_index: 0,
                content_index: 0,
                delta: "x".to_owned(),
            }),
            sequence_number: 1,
        };

        assert_eq!(
            to_value(&sequenced_event)
                .expect("a responses stream event must serialize")
                .pointer("/logprobs"),
            Some(&json!([]))
        );
    }
}
