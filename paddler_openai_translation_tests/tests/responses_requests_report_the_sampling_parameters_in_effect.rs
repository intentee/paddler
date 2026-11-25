use serde_json::json;

use paddler_openai_translation_tests::response_snapshots::ResponseSnapshots;

#[test]
fn responses_requests_report_the_sampling_parameters_in_effect() {
    let ResponseSnapshots { buffered, streamed } =
        ResponseSnapshots::of_buffered_and_streamed_requests()
            .expect("the response snapshots must be taken");

    assert_eq!([buffered.len(), streamed.len()], [1, 2]);

    for response_snapshot in buffered.iter().chain(&streamed) {
        assert_eq!(response_snapshot.get("temperature"), Some(&json!(0.25)));
        assert_eq!(response_snapshot.get("top_p"), Some(&json!(0.5)));
    }
}
