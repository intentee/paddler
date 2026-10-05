use std::io;

use iced_test::Error as InterfaceError;
use iced_test::Instruction;
use iced_test::runtime::Action;
use thiserror::Error;

use paddler_gui::message::Message;

#[derive(Debug, Error)]
pub enum GuiTestError {
    #[error("the task performed {action:?} instead of writing to the clipboard")]
    ActionIsNotAClipboardWrite { action: Box<Action<Message>> },
    #[error("the task performed {action:?} instead of producing a message")]
    ActionIsNotAMessage { action: Box<Action<Message>> },
    #[error("the cluster reported {message:?} instead of starting")]
    ClusterDidNotStart { message: Box<Message> },
    #[error("the emulator became ready without performing the expected action")]
    EmulatorReadyWithoutAction,
    #[error("the emulator stopped before becoming ready")]
    EmulatorStopped,
    #[error("the subscription executor could not start")]
    ExecutorUnavailable(#[source] io::Error),
    #[error("the emulated instruction failed: {instruction}")]
    InstructionFailed { instruction: Instruction },
    #[error("the interface did not respond as expected")]
    Interface(#[from] InterfaceError),
    #[error("the subscriptions ended before producing the expected message")]
    SubscriptionsEnded,
    #[error("the task ended before performing the expected action")]
    TaskEnded,
    #[error("the task performs no actions")]
    TaskPerformsNothing,
}
