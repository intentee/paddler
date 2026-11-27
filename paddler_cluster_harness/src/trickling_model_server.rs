use std::io;
use std::time::Duration;

use paddler_download_manager::constants::READ_TIMEOUT;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

const BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT: Duration =
    Duration::from_secs(READ_TIMEOUT.as_secs() / 2);

/// # Errors
///
/// Returns the [`io::Error`] of a model server that cannot listen on the loopback interface.
pub async fn trickling_model_server() -> io::Result<LocalHttpFixture> {
    LocalHttpFixture::start(FixtureResponse::EndlessTrickle {
        byte_interval: BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT,
    })
    .await
}
