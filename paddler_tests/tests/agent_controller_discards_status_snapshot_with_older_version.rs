use paddler::balancer::agent_controller_update_result::AgentControllerUpdateResult;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

#[test]
fn agent_controller_discards_status_snapshot_with_older_version() {
    let controller = make_agent_controller_without_remote_agent("test-agent");

    let initial = SlotAggregatedStatusSnapshot {
        version: 5,
        ..Default::default()
    };
    controller.update_from_slot_aggregated_status_snapshot(initial);

    let older = SlotAggregatedStatusSnapshot {
        version: 3,
        ..Default::default()
    };
    let result = controller.update_from_slot_aggregated_status_snapshot(older);

    assert!(matches!(
        result,
        AgentControllerUpdateResult::NoMeaningfulChanges
    ));
}
