use paddler::balancer::agent_controller_update_result::AgentControllerUpdateResult;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

#[test]
fn agent_controller_applies_newer_status_snapshot_with_model_path_change() {
    let controller = make_agent_controller_without_remote_agent("test-agent");

    let snapshot = SlotAggregatedStatusSnapshot {
        version: 1,
        model_path: Some("test_model".to_owned()),
        ..Default::default()
    };

    let result = controller.update_from_slot_aggregated_status_snapshot(snapshot);

    assert!(matches!(result, AgentControllerUpdateResult::Updated));
    assert_eq!(controller.get_model_path(), Some("test_model".to_owned()));
}
