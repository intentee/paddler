use paddler::balancer::agent_controller_update_result::AgentControllerUpdateResult;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

#[test]
fn agent_controller_status_snapshot_with_unchanged_values_reports_no_meaningful_changes() {
    let controller = make_agent_controller_without_remote_agent("test-agent");

    let snapshot = SlotAggregatedStatusSnapshot {
        version: 1,
        ..Default::default()
    };
    controller.update_from_slot_aggregated_status_snapshot(snapshot);

    let same_snapshot = SlotAggregatedStatusSnapshot {
        version: 1,
        ..Default::default()
    };
    let result = controller.update_from_slot_aggregated_status_snapshot(same_snapshot);

    assert!(matches!(
        result,
        AgentControllerUpdateResult::NoMeaningfulChanges
    ));
}
