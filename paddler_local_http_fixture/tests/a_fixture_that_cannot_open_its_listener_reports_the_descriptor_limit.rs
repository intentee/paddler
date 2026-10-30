use std::fs::File;
use std::io;
use std::os::fd::AsRawFd as _;

use nix::errno::Errno;
use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;
use nix::sys::resource::setrlimit;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test(flavor = "multi_thread")]
async fn a_fixture_that_cannot_open_its_listener_reports_the_descriptor_limit() {
    let lowest_free_descriptor = File::open("/dev/null")
        .expect("a probe descriptor must open")
        .as_raw_fd();
    let (_soft_descriptor_limit, hard_descriptor_limit) =
        getrlimit(Resource::RLIMIT_NOFILE).expect("the descriptor limit must be readable");

    setrlimit(
        Resource::RLIMIT_NOFILE,
        u64::try_from(lowest_free_descriptor).expect("descriptors are never negative"),
        hard_descriptor_limit,
    )
    .expect("the descriptor limit must be lowered");

    assert_eq!(
        LocalHttpFixture::start(FixtureResponse::Ok(Vec::new()))
            .await
            .err()
            .as_ref()
            .and_then(io::Error::raw_os_error)
            .map(Errno::from_raw),
        Some(Errno::EMFILE)
    );
}
