use anyhow::Result;
use anyhow::anyhow;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::image_url::ImageUrl;

use crate::agent::decoded_image::DecodedImage;

pub fn extract_images_from_conversation(
    conversation_history: &[ConversationMessage],
) -> Result<Vec<DecodedImage>> {
    let mut images = Vec::new();

    for message in conversation_history {
        for image_url in message.content.image_urls() {
            images.push(decode_image_from_data_uri(image_url)?);
        }
    }

    Ok(images)
}

fn decode_image_from_data_uri(image_url: &ImageUrl) -> Result<DecodedImage> {
    let url = &image_url.url;

    if !url.starts_with("data:") {
        return Err(anyhow!(
            "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
        ));
    }

    let after_data = url
        .strip_prefix("data:")
        .ok_or_else(|| anyhow!("Invalid data URI"))?;

    let (metadata, encoded_data) = after_data
        .split_once(',')
        .ok_or_else(|| anyhow!("Invalid data URI: missing comma separator"))?;

    let mime_type = metadata
        .split(';')
        .next()
        .ok_or_else(|| anyhow!("Invalid data URI: missing MIME type"))?
        .to_string();

    let data = BASE64_STANDARD.decode(encoded_data)?;

    Ok(DecodedImage { data, mime_type })
}
