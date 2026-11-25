use paddler_messaging::audio_format::AudioFormat;
use paddler_messaging::input_audio::InputAudio;

use crate::file_fixture_error::FileFixtureError;
use crate::fixture_base64::fixture_base64;

pub struct AudioFixture {
    pub fixture_name: &'static str,
    pub format: AudioFormat,
}

impl AudioFixture {
    /// # Errors
    ///
    /// Returns [`FileFixtureError::FixtureUnreadable`] when the audio fixture cannot be read.
    pub fn input_audio(&self) -> Result<InputAudio, FileFixtureError> {
        Ok(InputAudio {
            data: fixture_base64(self.fixture_name)?,
            format: self.format,
        })
    }
}
