use std::sync::Arc;

use llama_cpp_bindings::BareJsonToolCalls;
use llama_cpp_bindings::StreamingMarkers;

pub struct TokenClassification {
    pub bare_json_tool_calls: BareJsonToolCalls,
    pub streaming_markers: Arc<StreamingMarkers>,
}
