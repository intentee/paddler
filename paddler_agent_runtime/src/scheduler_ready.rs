use std::sync::Arc;

use paddler_messaging::model_metadata::ModelMetadata;

use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_request_preparer::SchedulerRequestPreparer;

pub struct SchedulerReady<TPreparation: PreparesSchedulerCommand> {
    pub model_metadata: ModelMetadata,
    pub request_preparer: Arc<SchedulerRequestPreparer<TPreparation>>,
}
