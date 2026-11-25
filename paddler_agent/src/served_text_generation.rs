use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::shuts_down::ShutsDown;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;

pub enum ServedTextGeneration {
    NotServed,
    Served(SchedulerHandle<GenerationRequestPreparer>),
}

impl ServedTextGeneration {
    pub fn forward(&self, agent_name: Option<&str>, request: TextGenerationRequest) {
        match self {
            Self::NotServed => TextGenerationError::report_text_generation_not_served(
                agent_name,
                request.generated_tokens_tx(),
            ),
            Self::Served(scheduler_handle) => scheduler_handle.request_preparer.prepare(request),
        }
    }
}

impl ShutsDown for ServedTextGeneration {
    async fn shut_down(self) {
        match self {
            Self::NotServed => {}
            Self::Served(scheduler_handle) => scheduler_handle.shut_down().await,
        }
    }
}
