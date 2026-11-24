use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::time::Duration;

use paddler_download_manager::constants::READ_TIMEOUT;

pub const BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT: Duration =
    Duration::from_secs(READ_TIMEOUT.as_secs() / 2);
pub const EPHEMERAL_LOOPBACK_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
pub const FAILED_RUN_EXIT_CODE: i32 = 1;
pub const LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);
pub const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);
pub const SMALL_DECISION_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();
pub const TYPESAFE_REQUEST_MODEL: &str = "test";
pub const UNENDING_GENERATION_CYCLE: &str = "apple banana cherry ";
