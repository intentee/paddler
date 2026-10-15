use paddler_messaging::request_params::decide_params::DecideParams;

use crate::system_one_response_layout::SystemOneResponseLayout;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslatedSystemOneRequest {
    pub decide_params: DecideParams,
    pub response_layout: SystemOneResponseLayout,
}
