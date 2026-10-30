use tokio::sync::mpsc::UnboundedSender;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;

pub struct NumberCommand {
    pub rejected_tx: UnboundedSender<u32>,
    pub value: u32,
}

impl SchedulerCommand for NumberCommand {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        send_result_or_warn(agent_name, &self.rejected_tx, self.value);
    }
}
