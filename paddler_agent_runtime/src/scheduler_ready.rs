use std::sync::Arc;

use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_request_preparer::SchedulerRequestPreparer;

pub struct SchedulerReady<TPreparation: PreparesSchedulerCommand> {
    pub request_preparer: Arc<SchedulerRequestPreparer<TPreparation>>,
}
