use serde_json::Value;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct ResponseSnapshotEvent {
    pub sequence_number: u64,
    pub response: Value,
}

#[derive(Clone, Debug)]
pub struct OutputItemEvent {
    pub sequence_number: u64,
    pub output_index: usize,
    pub item: Value,
}

#[derive(Clone, Debug)]
pub struct ContentPartEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub part: Value,
}

#[derive(Clone, Debug)]
pub struct TextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub delta: String,
}

#[derive(Clone, Debug)]
pub struct TextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct FunctionCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub delta: String,
}

#[derive(Clone, Debug)]
pub struct FunctionCallArgumentsDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub name: String,
    pub arguments: String,
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

    #[must_use]
    pub fn to_json(&self) -> Value {
        let event_type = self.event_name();

        match self {
            Self::Created(snapshot)
            | Self::InProgress(snapshot)
            | Self::Completed(snapshot)
            | Self::Failed(snapshot) => json!({
                "type": event_type,
                "sequence_number": snapshot.sequence_number,
                "response": snapshot.response,
            }),
            Self::OutputItemAdded(item_event) | Self::OutputItemDone(item_event) => json!({
                "type": event_type,
                "sequence_number": item_event.sequence_number,
                "output_index": item_event.output_index,
                "item": item_event.item,
            }),
            Self::ContentPartAdded(part_event) | Self::ContentPartDone(part_event) => json!({
                "type": event_type,
                "sequence_number": part_event.sequence_number,
                "item_id": part_event.item_id,
                "output_index": part_event.output_index,
                "content_index": part_event.content_index,
                "part": part_event.part,
            }),
            Self::OutputTextDelta(delta_event) => json!({
                "type": event_type,
                "sequence_number": delta_event.sequence_number,
                "item_id": delta_event.item_id,
                "output_index": delta_event.output_index,
                "content_index": delta_event.content_index,
                "delta": delta_event.delta,
                "logprobs": [],
            }),
            // Reasoning text events, unlike output-text events, do not carry a `logprobs` field.
            Self::ReasoningTextDelta(delta_event) => json!({
                "type": event_type,
                "sequence_number": delta_event.sequence_number,
                "item_id": delta_event.item_id,
                "output_index": delta_event.output_index,
                "content_index": delta_event.content_index,
                "delta": delta_event.delta,
            }),
            Self::OutputTextDone(done_event) => json!({
                "type": event_type,
                "sequence_number": done_event.sequence_number,
                "item_id": done_event.item_id,
                "output_index": done_event.output_index,
                "content_index": done_event.content_index,
                "text": done_event.text,
                "logprobs": [],
            }),
            Self::ReasoningTextDone(done_event) => json!({
                "type": event_type,
                "sequence_number": done_event.sequence_number,
                "item_id": done_event.item_id,
                "output_index": done_event.output_index,
                "content_index": done_event.content_index,
                "text": done_event.text,
            }),
            Self::FunctionCallArgumentsDelta(arguments_event) => json!({
                "type": event_type,
                "sequence_number": arguments_event.sequence_number,
                "item_id": arguments_event.item_id,
                "output_index": arguments_event.output_index,
                "delta": arguments_event.delta,
            }),
            Self::FunctionCallArgumentsDone(arguments_event) => json!({
                "type": event_type,
                "sequence_number": arguments_event.sequence_number,
                "item_id": arguments_event.item_id,
                "output_index": arguments_event.output_index,
                "name": arguments_event.name,
                "arguments": arguments_event.arguments,
            }),
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
    fn to_json_type_field_matches_event_name() {
        let event = ResponsesStreamEvent::Completed(ResponseSnapshotEvent {
            sequence_number: 7,
            response: json!({ "id": "resp_0" }),
        });

        let serialized = event.to_json();

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

        assert_eq!(event.to_json()["logprobs"], json!([]));
    }
}
