use serde::Deserialize;
use serde::Serialize;

use crate::audio_format::AudioFormat;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputAudio {
    pub data: String,
    pub format: AudioFormat,
}
