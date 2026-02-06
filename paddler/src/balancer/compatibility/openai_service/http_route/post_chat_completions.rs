use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use anyhow::anyhow;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::http_stream_from_agent::http_stream_from_agent;
use crate::balancer::unbounded_stream_from_agent::unbounded_stream_from_agent;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

#[derive(Deserialize)]
/// Although fields are same as in Paddler's conversation message for the moment,
/// it would be better if this struct stayed independent from ours just in case
/// to avoid any potential side effects in the future.
struct OpenAIMessage {
    content: String,
    role: String,
}

impl From<&OpenAIMessage> for ConversationMessage {
    fn from(openai_message: &OpenAIMessage) -> Self {
        ConversationMessage {
            content: openai_message.content.clone(),
            role: openai_message.role.clone(),
        }
    }
}

#[derive(Deserialize)]
struct OpenAICompletionRequestParams {
    max_completion_tokens: Option<i32>,
    messages: Vec<OpenAIMessage>,
    /// This parameter is ignored here, but is required by the OpenAI API.
    model: String,
    stream: Option<bool>,
}

#[derive(Clone)]
struct OpenAIStreamingResponseTransformer {
    model: String,
    system_fingerprint: String,
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
    type TransformedMessage = serde_json::Value;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> anyhow::Result<Self::TransformedMessage> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done),
            }) => Ok(json!({
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": current_timestamp(),
                "model": self.model,
                "system_fingerprint": self.system_fingerprint,
                "choices": [
                    {
                        "index": 0,
                        "delta": {},
                        "logprobs": null,
                        "finish_reason": "stop"
                    }
                ]
            })),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Token(token)),
            }) => Ok(json!({
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": current_timestamp(),
                "model": self.model,
                "system_fingerprint": self.system_fingerprint,
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "role": "assistant",
                            "content": token,
                        },
                        "logprobs": null,
                        "finish_reason": null
                    }
                ]
            })),
            _ => Ok(serde_json::to_value(&message)?),
        }
    }
}

#[derive(Clone)]
struct OpenAICombinedResponseTransformer {}

#[async_trait]
impl TransformsOutgoingMessage for OpenAICombinedResponseTransformer {
    type TransformedMessage = String;

    fn stringify(&self, message: &Self::TransformedMessage) -> anyhow::Result<String> {
        Ok(message.clone())
    }

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> anyhow::Result<Self::TransformedMessage> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done),
                ..
            }) => Ok("".to_string()),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Token(token)),
                ..
            }) => Ok(token),
            _ => Err(anyhow!("Unexpected message type: {:?}", message)),
        }
    }
}

#[post("/v1/chat/completions")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    let paddler_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: openai_params
            .messages
            .iter()
            .map(ConversationMessage::from)
            .collect(),
        enable_thinking: true,
        max_tokens: openai_params.max_completion_tokens.unwrap_or(2000),
        tools: vec![],
    };

    if openai_params.stream.unwrap_or(false) {
        http_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                model: openai_params.model.clone(),
                system_fingerprint: nanoid!(),
            },
        )
    } else {
        let combined_response = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAICombinedResponseTransformer {},
        )?
        .collect::<Vec<String>>()
        .await
        .join("");

        Ok(HttpResponse::Ok().json(json!({
          "id": nanoid!(),
          "object": "chat.completion",
          "created": current_timestamp(),
          "model": openai_params.model,
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": combined_response,
                "refusal": null,
                "annotations": []
              },
              "logprobs": null,
              "finish_reason": "stop"
            }
          ],
          "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
            "prompt_tokens_details": {
              "cached_tokens": 0,
              "audio_tokens": 0
            },
            "completion_tokens_details": {
              "reasoning_tokens": 0,
              "audio_tokens": 0,
              "accepted_prediction_tokens": 0,
              "rejected_prediction_tokens": 0
            }
          },
          "service_tier": "default"
        })))
    }
}
