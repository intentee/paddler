use serde_json::Error;
use serde_json::Value;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

pub trait SerializesTools {
    fn serialize_tools(&self, tools: &[Tool<Value>]) -> Result<String, Error>;
}
