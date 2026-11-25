use serde_json::Value;
use serde_json::json;

use paddler_openai_translation_tests::response_snapshots::ResponseSnapshots;

#[test]
fn responses_requests_report_their_status_and_a_resp_prefixed_id() {
    let ResponseSnapshots { buffered, streamed } =
        ResponseSnapshots::of_buffered_and_streamed_requests()
            .expect("the response snapshots must be taken");

    assert_eq!(
        buffered
            .iter()
            .chain(&streamed)
            .map(|response_snapshot| response_snapshot.get("status"))
            .collect::<Vec<_>>(),
        vec![
            Some(&json!("completed")),
            Some(&json!("in_progress")),
            Some(&json!("in_progress")),
        ]
    );

    for response_snapshot in buffered.iter().chain(&streamed) {
        assert!(
            response_snapshot
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|response_id| response_id.starts_with("resp_"))
        );
    }
}
