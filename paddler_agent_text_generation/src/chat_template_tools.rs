use serde_json::Map;
use serde_json::Value;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;

fn openai_function_tool(tool: &Tool<Value>) -> Value {
    let Tool::Function(FunctionCall {
        function:
            Function {
                name,
                description,
                parameters,
            },
    }) = tool;
    let mut function = Map::new();

    function.insert("name".to_owned(), Value::String(name.clone()));
    function.insert("description".to_owned(), Value::String(description.clone()));

    if let Parameters::Schema(schema) = parameters {
        function.insert("parameters".to_owned(), schema.clone());
    }

    Value::Object(Map::from_iter([
        ("type".to_owned(), Value::String("function".to_owned())),
        ("function".to_owned(), Value::Object(function)),
    ]))
}

pub struct ChatTemplateTools {
    pub json: String,
    pub tools: Value,
}

impl ChatTemplateTools {
    #[must_use]
    pub fn new(tools: &[Tool<Value>]) -> Self {
        let tools = Value::Array(tools.iter().map(openai_function_tool).collect());

        Self {
            json: tools.to_string(),
            tools,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;

    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;

    use super::ChatTemplateTools;

    fn tool(parameters: Parameters<Value>) -> Tool<Value> {
        Tool::Function(FunctionCall {
            function: Function {
                name: "get_weather".to_owned(),
                description: "Reports the weather".to_owned(),
                parameters,
            },
        })
    }

    #[test]
    fn a_tool_with_parameters_becomes_an_openai_function_with_its_schema() {
        let schema = json!({"type": "object", "properties": {"location": {"type": "string"}}});

        assert_eq!(
            ChatTemplateTools::new(&[tool(Parameters::Schema(schema.clone()))]).json,
            json!([{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Reports the weather",
                    "parameters": schema,
                },
            }])
            .to_string()
        );
    }

    #[test]
    fn a_tool_without_parameters_becomes_an_openai_function_without_them() {
        assert_eq!(
            ChatTemplateTools::new(&[tool(Parameters::Empty)]).json,
            json!([{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Reports the weather",
                },
            }])
            .to_string()
        );
    }
}
