use serde::Deserialize;

#[derive(Deserialize)]
pub enum IdentifiedMessage {
    Error { request_id: String },
    Response { request_id: String },
}

impl IdentifiedMessage {
    #[must_use]
    pub fn into_request_id(self) -> String {
        match self {
            Self::Error { request_id } | Self::Response { request_id } => request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use super::IdentifiedMessage;

    #[test]
    fn identifies_the_request_of_an_envelope_whose_contents_are_not_understood() {
        let request_ids = [
            json!({ "Error": { "request_id": "errored", "error": { "NoSuchError": {} } } }),
            json!({ "Response": { "request_id": "responded", "response": { "NoSuchResponse": {} } } }),
        ]
        .map(|envelope| {
            from_value::<IdentifiedMessage>(envelope)
                .map(IdentifiedMessage::into_request_id)
                .ok()
        });

        assert_eq!(
            request_ids,
            [Some("errored".to_owned()), Some("responded".to_owned())]
        );
    }
}
