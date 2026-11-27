use paddler_messaging::audio_format::AudioFormat;
use paddler_messaging::input_audio::InputAudio;

use crate::file_fixture_error::FileFixtureError;
use crate::fixture_base64::fixture_base64;

#[derive(Clone, Copy, Debug)]
pub enum AudioFixture {
    QuickBrownFoxWav,
}

impl AudioFixture {
    /// # Errors
    ///
    /// Returns [`FileFixtureError::FixtureUnreadable`] when the audio fixture cannot be read.
    pub fn input_audio(self) -> Result<InputAudio, FileFixtureError> {
        match self {
            Self::QuickBrownFoxWav => Ok(InputAudio {
                data: fixture_base64("quick_brown_fox.wav")?,
                format: AudioFormat::Wav,
            }),
        }
    }
}
