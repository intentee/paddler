use crate::tool_call_pipeline::ToolCallPipeline;

pub enum ToolCallPipelineBuildOutcome {
    Disabled,
    Ready(ToolCallPipeline),
    SchemaInvalid(String),
}
