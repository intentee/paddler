use llama_cpp_bindings_types::TokenUsage;
use serde_json::Value;
use serde_json::json;

use crate::compatibility::openai_service::openai_error::OpenAIError;

fn responses_usage_json(usage: &TokenUsage) -> Value {
    json!({
        "input_tokens": usage.prompt_tokens,
        "input_tokens_details": { "cached_tokens": usage.cached_prompt_tokens },
        "output_tokens": usage.completion_tokens(),
        "output_tokens_details": { "reasoning_tokens": usage.reasoning_tokens },
        "total_tokens": usage.total_tokens(),
    })
}

#[derive(Clone)]
pub struct ResponsesResponseBuilder {
    pub id: String,
    pub created_at: u64,
    pub model: String,
    pub instructions: Option<String>,
}

impl ResponsesResponseBuilder {
    // `usage` is intentionally absent here: the official `ResponseUsage` reference is not nullable, so the
    // in-progress and failed snapshots must omit it rather than emit `null`. Only `completed` adds it.
    fn base(&self, status: &str, output: &Value, error: &Value) -> Value {
        let instructions = self
            .instructions
            .as_ref()
            .map_or(Value::Null, |instructions| json!(instructions));

        json!({
            "id": self.id,
            "object": "response",
            "created_at": self.created_at,
            "status": status,
            "error": error,
            "incomplete_details": null,
            "instructions": instructions,
            "model": self.model,
            "tools": [],
            "output": output,
            "parallel_tool_calls": true,
            "metadata": {},
            "tool_choice": "auto",
            "temperature": 1,
            "top_p": 1,
            "text": { "format": { "type": "text" } }
        })
    }

    #[must_use]
    pub fn in_progress(&self) -> Value {
        self.base("in_progress", &json!([]), &Value::Null)
    }

    #[must_use]
    pub fn completed(&self, output: Vec<Value>, usage: &TokenUsage) -> Value {
        let mut response = self.base("completed", &Value::Array(output), &Value::Null);

        if let Some(object) = response.as_object_mut() {
            object.insert("usage".to_owned(), responses_usage_json(usage));
        }

        response
    }

    #[must_use]
    pub fn failed(&self, error: &OpenAIError) -> Value {
        self.base(
            "failed",
            &json!([]),
            &json!({ "code": "server_error", "message": error.message }),
        )
    }
}
