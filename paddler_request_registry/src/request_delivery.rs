#[derive(Debug, Eq, PartialEq)]
pub enum RequestDelivery {
    Delivered,
    ReceiverDropped,
    RequestNotRegistered,
}
