use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use super::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::transform_result::TransformResult;

#[derive(Default)]
pub struct IdentityTransformer;

impl IdentityTransformer {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl TransformsOutgoingMessage for IdentityTransformer {
    fn transform(&self, message: OutgoingMessage) -> Vec<TransformResult> {
        vec![TransformResult::serialized_from(&message)]
    }
}
