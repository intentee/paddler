use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

#[test]
fn agent_controller_status_snapshot_does_not_clobber_local_slots_processing() {
    let controller = make_agent_controller_without_remote_agent("test-agent");
    controller.slots_total.set(2);

    let baseline = SlotAggregatedStatusSnapshot {
        version: 5,
        slots_total: 2,
        ..Default::default()
    };
    controller.update_from_slot_aggregated_status_snapshot(baseline);
    assert_eq!(controller.slots_processing.get(), 0);

    controller.slots_processing.increment();
    assert_eq!(controller.slots_processing.get(), 1);

    let stale = SlotAggregatedStatusSnapshot {
        version: 7,
        slots_total: 2,
        slots_processing: 0,
        ..Default::default()
    };
    controller.update_from_slot_aggregated_status_snapshot(stale);

    assert_eq!(
        controller.slots_processing.get(),
        1,
        "stale snapshot from agent overwrote dispatcher's optimistic increment"
    );
}
