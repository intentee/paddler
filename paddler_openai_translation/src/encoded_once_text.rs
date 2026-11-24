use std::sync::Arc;
use std::sync::OnceLock;

use serde::Serialize;
use serde::Serializer;
use serde::ser::Error as _;
use serde_json::value::RawValue;
use serde_json::value::to_raw_value;

#[derive(Clone, Debug)]
pub struct EncodedOnceText {
    encoded: Arc<OnceLock<Box<RawValue>>>,
    text: Arc<str>,
}

impl EncodedOnceText {
    #[must_use]
    pub fn new(text: String) -> Self {
        Self {
            encoded: Arc::default(),
            text: Arc::from(text),
        }
    }
}

impl Serialize for EncodedOnceText {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        match self.encoded.get() {
            Some(encoded) => encoded.serialize(serializer),
            None => to_raw_value(&*self.text)
                .map_err(TSerializer::Error::custom)
                .and_then(|encoded| self.encoded.get_or_init(|| encoded).serialize(serializer)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::to_string;

    use super::EncodedOnceText;

    #[test]
    fn serializes_as_the_json_string_of_its_text() {
        assert_eq!(
            to_string(&EncodedOnceText::new("a \"quoted\"\nline".to_owned()))
                .expect("a text must serialize"),
            r#""a \"quoted\"\nline""#
        );
    }

    #[test]
    fn serializing_again_writes_the_first_encoding() {
        let encoded_once_text = EncodedOnceText::new("shared".to_owned());
        let first_serialization =
            to_string(&encoded_once_text).expect("the text must serialize the first time");

        assert_eq!(
            to_string(&encoded_once_text).expect("the text must serialize again"),
            first_serialization
        );
    }
}
