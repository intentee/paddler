use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeMap;

use crate::compatibility::openai_service::content_part_event::ContentPartEvent;
use crate::compatibility::openai_service::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::compatibility::openai_service::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::compatibility::openai_service::output_item_event::OutputItemEvent;
use crate::compatibility::openai_service::response_snapshot_event::ResponseSnapshotEvent;
use crate::compatibility::openai_service::text_delta_event::TextDeltaEvent;
use crate::compatibility::openai_service::text_done_event::TextDoneEvent;

const NO_LOGPROBS: [u8; 0] = [];

fn serialize_text_delta<TSerializeMap>(
    event: &mut TSerializeMap,
    delta_event: &TextDeltaEvent,
) -> Result<(), TSerializeMap::Error>
where
    TSerializeMap: SerializeMap,
{
    event.serialize_entry("sequence_number", &delta_event.sequence_number)?;
    event.serialize_entry("item_id", &delta_event.item_id)?;
    event.serialize_entry("output_index", &delta_event.output_index)?;
    event.serialize_entry("content_index", &delta_event.content_index)?;
    event.serialize_entry("delta", &delta_event.delta)
}

fn serialize_text_done<TSerializeMap>(
    event: &mut TSerializeMap,
    done_event: &TextDoneEvent,
) -> Result<(), TSerializeMap::Error>
where
    TSerializeMap: SerializeMap,
{
    event.serialize_entry("sequence_number", &done_event.sequence_number)?;
    event.serialize_entry("item_id", &done_event.item_id)?;
    event.serialize_entry("output_index", &done_event.output_index)?;
    event.serialize_entry("content_index", &done_event.content_index)?;
    event.serialize_entry("text", &done_event.text)
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
        let mut event = serializer.serialize_map(None)?;

        event.serialize_entry("type", self.event_name())?;

        match self {
            Self::Created(snapshot)
            | Self::InProgress(snapshot)
            | Self::Completed(snapshot)
            | Self::Failed(snapshot) => {
                event.serialize_entry("sequence_number", &snapshot.sequence_number)?;
                event.serialize_entry("response", &snapshot.response)?;
            }
            Self::OutputItemAdded(item_event) | Self::OutputItemDone(item_event) => {
                event.serialize_entry("sequence_number", &item_event.sequence_number)?;
                event.serialize_entry("output_index", &item_event.output_index)?;
                event.serialize_entry("item", &item_event.item)?;
            }
            Self::ContentPartAdded(part_event) | Self::ContentPartDone(part_event) => {
                event.serialize_entry("sequence_number", &part_event.sequence_number)?;
                event.serialize_entry("item_id", &part_event.item_id)?;
                event.serialize_entry("output_index", &part_event.output_index)?;
                event.serialize_entry("content_index", &part_event.content_index)?;
                event.serialize_entry("part", &part_event.part)?;
            }
            Self::OutputTextDelta(delta_event) => {
                serialize_text_delta(&mut event, delta_event)?;
                event.serialize_entry("logprobs", &NO_LOGPROBS)?;
            }
            Self::ReasoningTextDelta(delta_event) => {
                serialize_text_delta(&mut event, delta_event)?;
            }
            Self::OutputTextDone(done_event) => {
                serialize_text_done(&mut event, done_event)?;
                event.serialize_entry("logprobs", &NO_LOGPROBS)?;
            }
            Self::ReasoningTextDone(done_event) => {
                serialize_text_done(&mut event, done_event)?;
            }
            Self::FunctionCallArgumentsDelta(arguments_event) => {
                event.serialize_entry("sequence_number", &arguments_event.sequence_number)?;
                event.serialize_entry("item_id", &arguments_event.item_id)?;
                event.serialize_entry("output_index", &arguments_event.output_index)?;
                event.serialize_entry("delta", &arguments_event.delta)?;
            }
            Self::FunctionCallArgumentsDone(arguments_event) => {
                event.serialize_entry("sequence_number", &arguments_event.sequence_number)?;
                event.serialize_entry("item_id", &arguments_event.item_id)?;
                event.serialize_entry("output_index", &arguments_event.output_index)?;
                event.serialize_entry("name", &arguments_event.name)?;
                event.serialize_entry("arguments", &arguments_event.arguments)?;
            }
        }

        event.end()
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
