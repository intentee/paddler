use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use crate::transform_result::TransformResult;

pub trait TransformsOutgoingMessage {
    fn transform(&self, message: OutgoingMessage) -> Vec<TransformResult>;
}
