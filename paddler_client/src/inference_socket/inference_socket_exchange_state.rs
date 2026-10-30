use crate::inference_socket::open_inference_socket_exchange::OpenInferenceSocketExchange;

pub enum InferenceSocketExchangeState {
    Closed,
    Open(OpenInferenceSocketExchange),
}
