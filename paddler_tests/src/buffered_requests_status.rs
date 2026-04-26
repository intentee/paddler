use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

pub struct BufferedRequestsStatus;

impl BufferedRequestsStatus {
    pub fn count_is(expected_count: i32) -> impl Fn(&BufferedRequestManagerSnapshot) -> bool {
        move |snapshot| snapshot.buffered_requests_current == expected_count
    }
}
