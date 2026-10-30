use paddler_messaging::inference_client::message::Message as OutgoingMessage;

pub trait TransformsOutgoingMessage {
    type Output: Send + Sync + 'static;

    fn transform(&self, message: OutgoingMessage) -> Vec<Self::Output>;
}
