use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

pub const CA_BUNDLE_WITHOUT_CERTIFICATES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/ca_bundle_without_certificates.pem"
);
pub const EPHEMERAL_LOOPBACK_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
pub const FAILED_RUN_EXIT_CODE: i32 = 1;
pub const LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);
pub const TYPESAFE_REQUEST_MODEL: &str = "test";
pub const UNENDING_GENERATION_CYCLE: &str = "apple banana cherry ";
