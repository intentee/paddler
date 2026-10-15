use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::running_pipeline::RunningPipeline;

pub enum StartedPipeline {
    Cancelled,
    NothingToServe,
    Running {
        model_path: ModelPath,
        running_pipeline: RunningPipeline,
    },
}
