use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::content_part_event::ContentPartEvent;
use crate::compatibility::openai_service::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::compatibility::openai_service::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::compatibility::openai_service::output_item_event::OutputItemEvent;
use crate::compatibility::openai_service::response_snapshot_event::ResponseSnapshotEvent;
use crate::compatibility::openai_service::text_delta_event::TextDeltaEvent;
use crate::compatibility::openai_service::text_done_event::TextDoneEvent;

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
    #[serde(flatten)]
    payload: &'event TPayload,
}

fn serialize_typed<TPayload, TSerializer>(
    event_type: &'static str,
    payload: &TPayload,
    serializer: TSerializer,
) -> Result<TSerializer::Ok, TSerializer::Error>
where
    TPayload: Serialize,
    TSerializer: Serializer,
{
    TypedEvent {
        event_type,
        payload,
    }
    .serialize(serializer)
}

#[derive(Clone, Debug)]
pub enum ResponsesStreamEvent {
    Created(ResponseSnapshotEvent),
    InProgress(ResponseSnapshotEvent),
    OutputItemAdded(OutputItemEvent),
    OutputItemDone(OutputItemEvent),
    ContentPartAdded(ContentPartEvent),
    ContentPartDone(ContentPartEvent),
    OutputTextDelta(TextDeltaEvent),
    OutputTextDone(TextDoneEvent),
    ReasoningTextDelta(TextDeltaEvent),
    ReasoningTextDone(TextDoneEvent),
    FunctionCallArgumentsDelta(FunctionCallArgumentsDeltaEvent),
    FunctionCallArgumentsDone(FunctionCallArgumentsDoneEvent),
    Completed(ResponseSnapshotEvent),
    Incomplete(ResponseSnapshotEvent),
    Failed(ResponseSnapshotEvent),
}

impl ResponsesStreamEvent {
    #[must_use]
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::Created(_) => "response.created",
            Self::InProgress(_) => "response.in_progress",
            Self::OutputItemAdded(_) => "response.output_item.added",
            Self::OutputItemDone(_) => "response.output_item.done",
            Self::ContentPartAdded(_) => "response.content_part.added",
            Self::ContentPartDone(_) => "response.content_part.done",
            Self::OutputTextDelta(_) => "response.output_text.delta",
            Self::OutputTextDone(_) => "response.output_text.done",
            Self::ReasoningTextDelta(_) => "response.reasoning_text.delta",
            Self::ReasoningTextDone(_) => "response.reasoning_text.done",
            Self::FunctionCallArgumentsDelta(_) => "response.function_call_arguments.delta",
            Self::FunctionCallArgumentsDone(_) => "response.function_call_arguments.done",
            Self::Completed(_) => "response.completed",
            Self::Incomplete(_) => "response.incomplete",
            Self::Failed(_) => "response.failed",
        }
    }
}

impl Serialize for ResponsesStreamEvent {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let event_type = self.event_name();

        match self {
            Self::Created(snapshot)
            | Self::InProgress(snapshot)
            | Self::Completed(snapshot)
            | Self::Incomplete(snapshot)
            | Self::Failed(snapshot) => serialize_typed(event_type, snapshot, serializer),
            Self::OutputItemAdded(item_event) | Self::OutputItemDone(item_event) => {
                serialize_typed(event_type, item_event, serializer)
            }
            Self::ContentPartAdded(part_event) | Self::ContentPartDone(part_event) => {
                serialize_typed(event_type, part_event, serializer)
            }
            Self::OutputTextDelta(delta_event) => serialize_typed(
                event_type,
                &WithEmptyLogprobs {
                    event: delta_event,
                    logprobs: &[],
                },
                serializer,
            ),
            Self::ReasoningTextDelta(delta_event) => {
                serialize_typed(event_type, delta_event, serializer)
            }
            Self::OutputTextDone(done_event) => serialize_typed(
                event_type,
                &WithEmptyLogprobs {
                    event: done_event,
                    logprobs: &[],
                },
                serializer,
            ),
            Self::ReasoningTextDone(done_event) => {
                serialize_typed(event_type, done_event, serializer)
            }
            Self::FunctionCallArgumentsDelta(arguments_event) => {
                serialize_typed(event_type, arguments_event, serializer)
            }
            Self::FunctionCallArgumentsDone(arguments_event) => {
                serialize_typed(event_type, arguments_event, serializer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ResponseSnapshotEvent;
    use super::ResponsesStreamEvent;
    use super::TextDeltaEvent;

    #[test]
    fn reasoning_and_text_delta_carry_their_distinct_event_names_with_the_same_payload_shape() {
        let text_delta = ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            sequence_number: 4,
            item_id: "msg_0".to_owned(),
            output_index: 0,
            content_index: 0,
            delta: "hi".to_owned(),
        });
        let reasoning_delta = ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
            sequence_number: 4,
            item_id: "rs_0".to_owned(),
            output_index: 0,
            content_index: 0,
            delta: "hmm".to_owned(),
        });

        assert_eq!(text_delta.event_name(), "response.output_text.delta");
        assert_eq!(
            reasoning_delta.event_name(),
            "response.reasoning_text.delta"
        );
    }

    #[test]
    fn serialized_type_field_matches_event_name() {
        let event = ResponsesStreamEvent::Completed(ResponseSnapshotEvent {
            sequence_number: 7,
            response: json!({ "id": "resp_0" }),
        });

        let serialized =
            serde_json::to_value(&event).expect("a responses stream event must serialize");

        assert_eq!(serialized["type"], event.event_name());
        assert_eq!(serialized["sequence_number"], 7);
        assert_eq!(serialized["response"]["id"], "resp_0");
    }

    #[test]
    fn text_delta_includes_the_required_logprobs_array() {
        let event = ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            sequence_number: 1,
            item_id: "msg_0".to_owned(),
            output_index: 0,
            content_index: 0,
            delta: "x".to_owned(),
        });

        assert_eq!(
            serde_json::to_value(&event).expect("a responses stream event must serialize")["logprobs"],
            json!([])
        );
    }
}
