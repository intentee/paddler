use std::sync::Arc;

use crate::inference_socket::connection::Connection;

pub enum ConnectionSlot {
    Connected(Arc<Connection>),
    Empty,
}
