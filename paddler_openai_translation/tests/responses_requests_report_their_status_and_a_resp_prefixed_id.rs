use crate::first_response_snapshots::FirstResponseSnapshots;

#[test]
fn responses_requests_report_their_status_and_a_resp_prefixed_id() {
    let FirstResponseSnapshots { buffered, streamed } =
        FirstResponseSnapshots::of_buffered_and_streamed_requests();

    assert_eq!(buffered["status"], "completed");
    assert_eq!(streamed["status"], "in_progress");

    for response_snapshot in [buffered, streamed] {
        assert!(
            response_snapshot["id"]
                .as_str()
                .is_some_and(|response_id| response_id.starts_with("resp_"))
        );
    }
}
