use std::pin::Pin;

use futures_util::Stream;
use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use crate::Result;

pub type BufferedRequestsStream =
    Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>;
