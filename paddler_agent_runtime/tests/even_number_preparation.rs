use tokio::sync::mpsc::UnboundedSender;

use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;

use crate::number_command::NumberCommand;

pub struct EvenNumberPreparation {
    pub rejected_tx: UnboundedSender<u32>,
}

impl PreparesSchedulerCommand for EvenNumberPreparation {
    type Command = NumberCommand;
    type Request = u32;

    fn prepare_scheduler_command(
        &self,
        _agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        request.is_multiple_of(2).then(|| NumberCommand {
            rejected_tx: self.rejected_tx.clone(),
            value: request,
        })
    }
}
