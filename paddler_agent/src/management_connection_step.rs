use futures_util::stream::BoxStream;

use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;

use crate::management_connection_end::ManagementConnectionEnd;

pub enum ManagementConnectionStep {
    Continue,
    Deregister,
    End(ManagementConnectionEnd),
    StreamResponses(BoxStream<'static, ManagementJsonRpcMessage>),
    Write(ManagementJsonRpcMessage),
}
