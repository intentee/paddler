use serde::Deserialize;
use serde::Serialize;

use crate::image_url::ImageUrl;
use crate::input_audio::InputAudio;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, tag = "type")]
pub enum ConversationMessageContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    #[serde(rename = "input_audio")]
    InputAudio { input_audio: InputAudio },
}
