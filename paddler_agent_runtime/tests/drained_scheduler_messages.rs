use tokio::sync::mpsc::UnboundedReceiver;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::number_command::NumberCommand;

pub fn drained_scheduler_messages(
    mut scheduler_message_rx: UnboundedReceiver<SchedulerMessage<NumberCommand>>,
) -> Vec<SchedulerMessage<NumberCommand>> {
    let mut scheduler_messages = Vec::new();

    while let Ok(scheduler_message) = scheduler_message_rx.try_recv() {
        scheduler_messages.push(scheduler_message);
    }

    scheduler_messages
}
