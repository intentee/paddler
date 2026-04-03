use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use log::error;
use log::warn;
use minijinja::context;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::media_marker::MediaMarker;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::sync::mpsc;

use crate::agent::continuous_batch_grammar_resolver::resolve_grammar;
use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::grammar_sampler::GrammarSampler;
use crate::decoded_image::DecodedImage;
use crate::decoded_image_error::DecodedImageError;

pub enum PreparedConversationHistoryRequest {
    TextPrompt {
        raw_prompt: String,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
    },
    MultimodalPrompt {
        raw_prompt: String,
        images: Vec<DecodedImage>,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
    },
}

pub fn prepare_conversation_history_request(
    params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    scheduler_context: &ContinuousBatchSchedulerContext,
) -> Result<PreparedConversationHistoryRequest> {
    let ContinueFromConversationHistoryParams {
        add_generation_prompt,
        enable_thinking,
        grammar,
        conversation_history,
        max_tokens,
        tools,
    } = params;

    let grammar_sampler = resolve_grammar(grammar.as_ref(), enable_thinking, generated_tokens_tx)?;

    let image_resize_to_fit = scheduler_context.inference_parameters.image_resize_to_fit;

    let images = conversation_history
        .extract_image_urls()
        .iter()
        .map(|image_url| {
            DecodedImage::from_data_uri(image_url)
                .and_then(|image| image.converted_to_png_if_necessary(image_resize_to_fit))
                .and_then(|image| image.resized_to_fit(image_resize_to_fit))
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

    let media_marker = MediaMarker::new(mtmd_default_marker().to_owned());
    let chat_template_messages = conversation_history.replace_images_with_marker(&media_marker);

    let raw_prompt = scheduler_context
        .chat_template_renderer
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
        });
    }

    Ok(PreparedConversationHistoryRequest::TextPrompt {
        raw_prompt,
        max_tokens,
        grammar_sampler,
    })
}
