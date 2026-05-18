use crate::connect_address_field::ConnectAddressField;
use crate::slot_count_field::SlotCountField;

#[derive(Default)]
pub struct JoinBalancerFormData {
    pub agent_name: String,
    pub balancer_address: ConnectAddressField,
    pub slots_count: SlotCountField,
}
