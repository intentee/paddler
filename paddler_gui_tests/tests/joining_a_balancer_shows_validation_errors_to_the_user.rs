use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::connect_address_field::ConnectAddressField;
use paddler_gui::join_balancer_form_data::JoinBalancerFormData;
use paddler_gui::slot_count_field::SlotCountField;
use paddler_gui::ui::view_join_balancer_form::view_join_balancer_form;

#[test]
fn the_cluster_address_and_slots_errors_render_under_their_inputs_when_set() -> Result<()> {
    let data = JoinBalancerFormData {
        balancer_address: ConnectAddressField::Invalid {
            raw: String::new(),
            error: "Cluster address is required.".to_owned(),
        },
        slots_count: SlotCountField::Invalid {
            raw: String::new(),
            error: "Number of slots is required.".to_owned(),
        },
        ..JoinBalancerFormData::default()
    };
    let mut simulator = simulator(view_join_balancer_form(&data));

    if simulator.find("Cluster address is required.").is_err() {
        bail!("expected cluster address error to render");
    }
    if simulator.find("Number of slots is required.").is_err() {
        bail!("expected slots error to render");
    }

    Ok(())
}
