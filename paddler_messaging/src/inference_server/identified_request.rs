use serde::Deserialize;

#[derive(Deserialize)]
pub enum IdentifiedRequest {
    Request { id: String },
}
