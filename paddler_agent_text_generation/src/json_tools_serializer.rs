use serde_json::Error;
use serde_json::Value;
use serde_json::to_string;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

use crate::serializes_tools::SerializesTools;

pub struct JsonToolsSerializer;

impl SerializesTools for JsonToolsSerializer {
    fn serialize_tools(&self, tools: &[Tool<Value>]) -> Result<String, Error> {
        to_string(tools)
    }
}
