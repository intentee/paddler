use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

pub fn buffered_request_count_is(
    expected_count: u64,
) -> impl Fn(&BufferedRequestManagerSnapshot) -> bool {
    move |snapshot| snapshot.buffered_requests_current == expected_count
}
