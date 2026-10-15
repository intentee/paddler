use std::mem::Discriminant;
use std::mem::discriminant;

use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;

use crate::management_connection_step::ManagementConnectionStep;

impl ManagementConnectionStep {
    pub fn written_message(&self) -> Result<&ManagementJsonRpcMessage, Discriminant<Self>> {
        match self {
            Self::Write(management_json_rpc_message) => Ok(management_json_rpc_message),
            unwritten_step => Err(discriminant(unwritten_step)),
        }
    }
}
