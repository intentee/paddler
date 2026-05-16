use iced::futures::channel::mpsc::Sender;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;

use crate::drive_agent_stream_inner::drive_agent_stream_inner;
use crate::message::Message;

pub async fn drive_agent_stream(params: AgentRunnerParams, output: Sender<Message>) {
    let mut runner = AgentRunner::start(params);

    let snapshot_source = runner.slot_aggregated_status.clone();
    let completion_future = runner.wait_for_completion();

    drive_agent_stream_inner(snapshot_source, completion_future, output).await;
}
