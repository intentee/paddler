use crate::address_field::AddressField;
use crate::slot_count_field::SlotCountField;

#[derive(Default)]
pub struct JoinBalancerFormData {
    pub agent_name: String,
    pub balancer_address: AddressField,
    pub slots_count: SlotCountField,
}
