use iced_test::runtime::Task;
use iced_test::simulator;

use paddler_gui::app::App;
use paddler_gui::home_message::HomeMessage;
use paddler_gui::join_balancer_form_message::JoinBalancerFormMessage;
use paddler_gui::message::Message;
use paddler_gui::start_balancer_form_message::StartBalancerFormMessage;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::gui_test_error::GuiTestError;
use crate::started_cluster::StartedCluster;
use crate::task_actions::TaskActions;

pub struct AppDriver {
    app: App,
}

impl AppDriver {
    pub async fn open() -> Result<Self, GuiTestError> {
        let (mut app, boot_task) = App::new();
        let boot_message = TaskActions::of(boot_task)?.next_message().await?;
        let _event_loop_ready_task = app.update(boot_message);

        Ok(Self { app })
    }

    pub fn click(&mut self, label: &str) -> Result<Task<Message>, GuiTestError> {
        self.messages_from_clicking(label).map(|messages| {
            Task::batch(messages.into_iter().map(|message| self.app.update(message)))
        })
    }

    pub async fn deliver_until<TPredicate>(
        &mut self,
        task_actions: &mut TaskActions,
        is_awaited_message: TPredicate,
    ) -> Result<(), GuiTestError>
    where
        TPredicate: Fn(&Message) -> bool,
    {
        loop {
            let message = task_actions.next_message().await?;
            let is_awaited = is_awaited_message(&message);
            let _follow_up_task = self.app.update(message);

            if is_awaited {
                return Ok(());
            }
        }
    }

    pub fn find(&self, text: &str) -> Result<(), GuiTestError> {
        simulator(self.app.view())
            .find(text)
            .map(|_found_target| ())
            .map_err(GuiTestError::from)
    }

    pub fn join_cluster(
        &mut self,
        balancer_address: String,
    ) -> Result<Task<Message>, GuiTestError> {
        for message in [
            Message::Home(HomeMessage::JoinBalancer),
            Message::JoinBalancerForm(JoinBalancerFormMessage::SetBalancerAddress(
                balancer_address,
            )),
            Message::JoinBalancerForm(JoinBalancerFormMessage::SetSlotsCount("1".to_owned())),
        ] {
            let _form_task = self.app.update(message);
        }

        self.click("Connect")
    }

    pub fn messages_from_clicking(&self, label: &str) -> Result<Vec<Message>, GuiTestError> {
        let mut interface = simulator(self.app.view());
        let click_result = interface.click(label);
        let messages = interface.into_messages().collect();

        click_result
            .map(|_clicked_target| messages)
            .map_err(GuiTestError::from)
    }

    pub fn confirm_cluster_start(
        &mut self,
        web_admin_panel_address: String,
    ) -> Result<TaskActions, GuiTestError> {
        for message in [
            Message::Home(HomeMessage::StartBalancer),
            Message::StartBalancerForm(StartBalancerFormMessage::SetBalancerAddress(
                EPHEMERAL_LOOPBACK_ADDR.to_string(),
            )),
            Message::StartBalancerForm(StartBalancerFormMessage::SetInferenceAddress(
                EPHEMERAL_LOOPBACK_ADDR.to_string(),
            )),
            Message::StartBalancerForm(StartBalancerFormMessage::SetWebAdminPanelAddress(
                web_admin_panel_address,
            )),
            Message::StartBalancerForm(StartBalancerFormMessage::ToggleAddModelLater(true)),
        ] {
            let _form_task = self.app.update(message);
        }

        TaskActions::of(self.app.update(Message::StartBalancerForm(
            StartBalancerFormMessage::Confirm,
        )))
    }

    pub async fn start_cluster(
        &mut self,
        web_admin_panel_address: String,
    ) -> Result<StartedCluster, GuiTestError> {
        let mut messages = self.confirm_cluster_start(web_admin_panel_address)?;

        match messages.next_message().await? {
            balancer_started @ Message::BalancerStarted { addresses, .. } => {
                let _running_task = self.app.update(balancer_started);

                Ok(StartedCluster {
                    addresses,
                    messages,
                })
            }
            message => Err(GuiTestError::ClusterDidNotStart {
                message: Box::new(message),
            }),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.app.update(message)
    }
}
