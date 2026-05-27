use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

pub fn assert_count(expected_count: i32) -> impl Fn(&BufferedRequestManagerSnapshot) -> bool {
    move |snapshot| snapshot.buffered_requests_current == expected_count
}
