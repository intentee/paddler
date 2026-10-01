use futures::StreamExt as _;
use futures::stream::BoxStream;
use iced_test::runtime::Action;
use iced_test::runtime::Task;
use iced_test::runtime::clipboard::Action as ClipboardAction;
use iced_test::runtime::task::into_stream;

use paddler_gui::message::Message;

use crate::gui_test_error::GuiTestError;

pub struct TaskActions {
    actions: BoxStream<'static, Action<Message>>,
}

impl TaskActions {
    pub fn of(task: Task<Message>) -> Result<Self, GuiTestError> {
        into_stream(task)
            .map(|actions| Self { actions })
            .ok_or(GuiTestError::TaskPerformsNothing)
    }

    pub async fn next_clipboard_write(&mut self) -> Result<String, GuiTestError> {
        match self.actions.next().await {
            Some(Action::Clipboard(ClipboardAction::Write { contents, .. })) => Ok(contents),
            Some(action) => Err(GuiTestError::ActionIsNotAClipboardWrite {
                action: Box::new(action),
            }),
            None => Err(GuiTestError::TaskEnded),
        }
    }

    pub async fn next_message(&mut self) -> Result<Message, GuiTestError> {
        match self.actions.next().await {
            Some(Action::Output(message)) => Ok(message),
            Some(action) => Err(GuiTestError::ActionIsNotAMessage {
                action: Box::new(action),
            }),
            None => Err(GuiTestError::TaskEnded),
        }
    }
}
