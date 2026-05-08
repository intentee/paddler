use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use paddler_client::ClientInference;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::handler::Handler;
use super::load_tool::load_tool;
use super::thinking_mode::ThinkingMode;
use super::value_parser::parse_inference_url::parse_inference_url;
use crate::chat_session::ChatSession;

#[derive(Parser)]
pub struct Prompt {
    #[arg(long, value_parser = parse_inference_url)]
    /// Address of the inference server (e.g. 127.0.0.1:8061)
    inference_addr: Url,

    #[arg(long)]
    /// Maximum number of tokens to generate
    max_tokens: i32,

    #[arg(long, value_enum)]
    /// Whether chain-of-thought thinking is on or off
    thinking: ThinkingMode,

    #[arg(long, action = clap::ArgAction::Append)]
    /// Path to a JSON file describing one tool (repeatable)
    tool: Vec<PathBuf>,

    /// Prompt to send to the model
    message: String,
}

#[async_trait]
impl Handler for Prompt {
    async fn handle(&self, shutdown: CancellationToken) -> Result<()> {
        let tools = self
            .tool
            .iter()
            .map(|path| load_tool(path))
            .collect::<Result<Vec<_>>>()?;

        let request = ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(self.message.clone()),
                role: "user".to_owned(),
            }]),
            enable_thinking: self.thinking.is_enabled(),
            grammar: None,
            max_tokens: self.max_tokens,
            parse_tool_calls: !tools.is_empty(),
            tools,
        };

        let http_client = Client::new();
        let inference = ClientInference::new(&self.inference_addr, &http_client, 1);
        let stream = inference
            .post_continue_from_conversation_history(&request)
            .await?;

        ChatSession::new(stream, shutdown).run().await
    }
}
