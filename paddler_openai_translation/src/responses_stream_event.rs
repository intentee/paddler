use crate::content_part_event::ContentPartEvent;
use crate::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::output_item_event::OutputItemEvent;
use crate::response_snapshot_event::ResponseSnapshotEvent;
use crate::text_delta_event::TextDeltaEvent;
use crate::text_done_event::TextDoneEvent;

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

#[cfg(test)]
mod tests {
    use super::ResponsesStreamEvent;
    use super::TextDeltaEvent;

    #[test]
    fn reasoning_and_text_delta_carry_their_distinct_event_names() {
        let text_delta = ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            item_id: "msg_0".to_owned(),
            output_index: 0,
            content_index: 0,
            delta: "hi".to_owned(),
        });
        let reasoning_delta = ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
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
}
