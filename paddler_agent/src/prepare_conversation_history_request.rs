use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use log::error;
use log::warn;
use minijinja::context;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::media_marker::MediaMarker;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::sync::mpsc;

use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::decoded_image::DecodedImage;
use crate::decoded_image_error::DecodedImageError;
use crate::prepared_conversation_history_request::PreparedConversationHistoryRequest;
use crate::resolve_grammar::resolve_grammar;

fn require_renderer_for_generation(
    chat_template_renderer: Option<&Arc<ChatTemplateRenderer>>,
    agent_name: Option<&str>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<Arc<ChatTemplateRenderer>> {
    chat_template_renderer.map_or_else(
        || {
            let message = format!(
                "{agent_name:?}: token generation is disabled because this agent is running in embeddings-only mode"
            );

            error!("{message}");

            if generated_tokens_tx
                .send(GeneratedTokenResult::TokenGenerationDisabled(
                    message.clone(),
                ))
                .is_err()
            {
                warn!("{agent_name:?}: failed to send result to client (receiver dropped)");
            }

            Err(anyhow!(message))
        },
        |chat_template_renderer| Ok(chat_template_renderer.clone()),
    )
}

pub fn prepare_conversation_history_request(
    ContinueFromConversationHistoryParams {
        add_generation_prompt,
        enable_thinking,
        grammar,
        conversation_history,
        max_tokens,
        parse_tool_calls,
        tools,
    }: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    scheduler_context: &ContinuousBatchSchedulerContext,
) -> Result<PreparedConversationHistoryRequest> {
    let grammar_sampler = resolve_grammar(grammar.as_ref(), enable_thinking, generated_tokens_tx)?;

    let image_resize_to_fit = scheduler_context.inference_parameters.image_resize_to_fit;

    let images = conversation_history
        .extract_image_urls()
        .iter()
        .map(|image_url| {
            DecodedImage::from_data_uri(image_url)
                .and_then(|image| image.prepared_for_inference(image_resize_to_fit))
        })
        .collect::<Result<Vec<DecodedImage>, DecodedImageError>>()
        .map_err(|err| {
            let message = format!(
                "{:?}: failed to decode images: {err}",
                scheduler_context.agent_name
            );

            error!("{message}");

            if generated_tokens_tx
                .send(GeneratedTokenResult::ImageDecodingFailed(message.clone()))
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    scheduler_context.agent_name
                );
            }

            anyhow!(message)
        })?;

    let media_marker = MediaMarker::new(mtmd_default_marker()?.to_owned());
    let chat_template_messages = conversation_history.replace_images_with_marker(&media_marker);

    let chat_template_renderer = require_renderer_for_generation(
        scheduler_context.chat_template_renderer.as_ref(),
        scheduler_context.agent_name.as_deref(),
        generated_tokens_tx,
    )?;

    let raw_prompt = chat_template_renderer
        .render(context! {
            add_generation_prompt,
            bos_token => scheduler_context.token_bos_str,
            enable_thinking,
            eos_token => scheduler_context.token_eos_str,
            messages => chat_template_messages.messages,
            nl_token => scheduler_context.token_nl_str,
            tools => tools,
        })
        .map_err(|err| {
            let message = format!(
                "{:?}: failed to render chat template: {err:?}",
                scheduler_context.agent_name
            );

            error!("{message}");

            if generated_tokens_tx
                .send(GeneratedTokenResult::ChatTemplateError(message.clone()))
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    scheduler_context.agent_name
                );
            }

            anyhow!(message)
        })?;

    let has_images = !images.is_empty();
    let has_multimodal_context = scheduler_context.multimodal_context.is_some();

    if has_images && !has_multimodal_context {
        let message = format!(
            "{:?}: received images but model does not support multimodal input",
            scheduler_context.agent_name
        );

        error!("{message}");

        if generated_tokens_tx
            .send(GeneratedTokenResult::MultimodalNotSupported(
                message.clone(),
            ))
            .is_err()
        {
            warn!(
                "{:?}: failed to send result to client (receiver dropped)",
                scheduler_context.agent_name
            );
        }

        return Err(anyhow!(message));
    }

    if has_images {
        return Ok(PreparedConversationHistoryRequest::MultimodalPrompt {
            raw_prompt,
            images,
            max_tokens,
            grammar_sampler,
            parse_tool_calls,
            tools,
        });
    }

    Ok(PreparedConversationHistoryRequest::TextPrompt {
        raw_prompt,
        max_tokens,
        grammar_sampler,
        parse_tool_calls,
        tools,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::require_renderer_for_generation;
    use crate::chat_template_renderer::ChatTemplateRenderer;

    #[test]
    fn returns_the_renderer_when_one_is_present() -> Result<()> {
        let chat_template_renderer = Arc::new(ChatTemplateRenderer::new(ChatTemplate {
            content: "{{ messages }}".to_owned(),
        })?);
        let (generated_tokens_tx, _generated_tokens_rx) = mpsc::unbounded_channel();

        let returned_renderer = require_renderer_for_generation(
            Some(&chat_template_renderer),
            Some("agent"),
            &generated_tokens_tx,
        )?;

        assert!(Arc::ptr_eq(&returned_renderer, &chat_template_renderer));

        Ok(())
    }

    #[test]
    fn rejects_with_token_generation_disabled_when_no_renderer_is_present() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let result = require_renderer_for_generation(None, Some("agent"), &generated_tokens_tx);

        assert!(result.is_err());
        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::TokenGenerationDisabled(_))
        ));
    }
}
