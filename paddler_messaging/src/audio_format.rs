use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AudioFormat {
    #[serde(rename = "mp3")]
    Mp3,
    #[serde(rename = "wav")]
    Wav,
}
