use crate::first_response_snapshots::FirstResponseSnapshots;

#[test]
fn responses_requests_report_the_sampling_parameters_in_effect() {
    let FirstResponseSnapshots { buffered, streamed } =
        FirstResponseSnapshots::of_buffered_and_streamed_requests();

    for response_snapshot in [buffered, streamed] {
        assert_eq!(response_snapshot["temperature"], 0.25);
        assert_eq!(response_snapshot["top_p"], 0.5);
    }
}
