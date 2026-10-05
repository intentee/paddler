use futures::StreamExt as _;
use futures::channel::mpsc;
use iced_test::core::Event;
use iced_test::core::event::Status;
use iced_test::core::window::Id as WindowId;
use iced_test::futures::Runtime;
use iced_test::futures::backend::default::Executor;
use iced_test::futures::subscription::Event as SubscriptionEvent;
use iced_test::futures::subscription::into_recipes;

use paddler_gui::app::App;
use paddler_gui::message::Message;

use crate::gui_test_error::GuiTestError;

pub struct AppSubscriptions {
    messages: mpsc::Receiver<Message>,
    runtime: Runtime<Executor, mpsc::Sender<Message>, Message>,
    window: WindowId,
}

impl AppSubscriptions {
    pub fn of(app: &App) -> Result<Self, GuiTestError> {
        let executor = Executor::new().map_err(GuiTestError::ExecutorUnavailable)?;
        let (message_tx, messages) = mpsc::channel(0);
        let mut runtime = Runtime::new(executor, message_tx);
        let recipes = into_recipes(runtime.enter(|| app.subscription()));

        runtime.track(recipes);

        Ok(Self {
            messages,
            runtime,
            window: WindowId::unique(),
        })
    }

    pub fn broadcast_unhandled(&mut self, event: Event) {
        self.runtime.broadcast(SubscriptionEvent::Interaction {
            window: self.window,
            event,
            status: Status::Ignored,
        });
    }

    pub fn next_message(&mut self) -> Result<Message, GuiTestError> {
        let messages = &mut self.messages;

        self.runtime
            .block_on(messages.next())
            .ok_or(GuiTestError::SubscriptionsEnded)
    }
}
