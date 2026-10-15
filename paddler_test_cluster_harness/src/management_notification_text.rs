use serde_json::Error;
use serde_json::to_string;

use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;

pub fn management_notification_text(
    notification: ManagementJsonRpcNotification,
) -> Result<String, Error> {
    to_string(&ManagementJsonRpcMessage::Notification(notification))
}
