use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::input_audio::InputAudio;

#[derive(Debug, Eq, PartialEq)]
pub enum ConversationMedia {
    Audio(InputAudio),
    Image(ImageUrl),
}

impl ConversationMedia {
    #[must_use]
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }
}
