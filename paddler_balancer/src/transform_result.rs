use bytes::Bytes;
use log::error;
use serde::Serialize;
use serde_json::to_string;

#[derive(Debug, Eq, PartialEq)]
pub enum TransformResult {
    Chunk(String),
    Discard,
    Error(String),
}

impl TransformResult {
    #[must_use]
    pub fn serialized_from<TMessage: Serialize>(message: &TMessage) -> Self {
        match to_string(message) {
            Ok(serialized) => Self::Chunk(serialized),
            Err(serialization_error) => {
                error!(
                    "Unable to serialize a message relayed to the client: {serialization_error}"
                );

                Self::Discard
            }
        }
    }

    #[must_use]
    pub fn into_ndjson_line(self) -> Option<Bytes> {
        match self {
            Self::Chunk(mut line) | Self::Error(mut line) => {
                line.push('\n');

                Some(Bytes::from(line))
            }
            Self::Discard => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use log::Level;
    use serde_json::to_string;

    use paddler_capturing_logger::captured_record::CapturedRecord;
    use paddler_capturing_logger::capturing_logger::CapturingLogger;

    use super::TransformResult;

    #[test]
    fn a_message_that_cannot_be_serialized_is_discarded() {
        CapturingLogger::install();

        let mut message_with_a_non_string_key = BTreeMap::new();

        message_with_a_non_string_key.insert(vec![0_u8], 0_u8);

        assert_eq!(
            TransformResult::serialized_from(&message_with_a_non_string_key),
            TransformResult::Discard
        );
        assert!(
            CapturingLogger::records_for("paddler_balancer::transform_result").contains(
                &CapturedRecord {
                    level: Level::Error,
                    message: format!(
                        "Unable to serialize a message relayed to the client: {}",
                        to_string(&message_with_a_non_string_key)
                            .expect_err("a map with a non-string key must not serialize")
                    ),
                    target: "paddler_balancer::transform_result".to_owned(),
                }
            )
        );
    }
}
