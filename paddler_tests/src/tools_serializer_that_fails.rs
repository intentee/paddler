use serde::ser::Error as _;
use serde_json::Error;
use serde_json::Value;

use paddler_agent_text_generation::serializes_tools::SerializesTools;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

pub struct ToolsSerializerThatFails;

impl SerializesTools for ToolsSerializerThatFails {
    fn serialize_tools(&self, _tools: &[Tool<Value>]) -> Result<String, Error> {
        Err(Error::custom("tools cannot be serialized"))
    }
}
