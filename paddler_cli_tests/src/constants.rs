use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;

use paddler_bootstrap::constants::LLAMA_CPP_MAX_SEQUENCES;

pub const CLAP_USAGE_ERROR_EXIT_CODE: i32 = 2;
pub const MISSING_BINARY_PATH: &str = "/paddler/definitely/missing/paddler";
pub const SERVING_AGENT_NAME: &str = "serving-agent";
pub const SERVING_AGENT_SLOTS: u16 = 1;
pub const SLOTS_BEYOND_WHAT_LLAMA_CPP_CAN_SERVE: u16 = LLAMA_CPP_MAX_SEQUENCES + 1;
pub const UNREACHABLE_MANAGEMENT_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1));
