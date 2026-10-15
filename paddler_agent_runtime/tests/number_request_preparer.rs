use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;

use crate::even_number_preparation::EvenNumberPreparation;
use crate::number_command::NumberCommand;

pub struct NumberRequestPreparer {
    pub rejected_rx: UnboundedReceiver<u32>,
    pub request_preparer: Arc<SchedulerRequestPreparer<EvenNumberPreparation>>,
    pub scheduler_message_rx: Receiver<SchedulerMessage<NumberCommand>>,
}

impl NumberRequestPreparer {
    pub fn new() -> Self {
        let (rejected_tx, rejected_rx) = unbounded_channel();
        let (scheduler_message_tx, scheduler_message_rx) = channel();

        Self {
            rejected_rx,
            request_preparer: Arc::new(SchedulerRequestPreparer {
                agent_name: Some("agent".to_owned()),
                preparation: EvenNumberPreparation { rejected_tx },
                preparation_tasks: TaskTracker::new(),
                scheduler_message_tx,
            }),
            scheduler_message_rx,
        }
    }
}
