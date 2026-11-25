use serde::Deserialize;

#[derive(Deserialize)]
pub enum TokenizerReferenceInputNormalization {
    #[serde(rename = "NFC")]
    Nfc,
}
