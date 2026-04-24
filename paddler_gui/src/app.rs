use std::mem;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;

use iced::Bottom;
use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Right;
use iced::Subscription;
use iced::Task;
use iced::futures::SinkExt;
use iced::keyboard;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::operation;
use iced::widget::stack;
use iced::window;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_bootstrap::shutdown_signal::wait_for_shutdown_signal;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::agent_running_handler;
use crate::current_screen::CurrentScreen;
use crate::home_data::HomeData;
use crate::home_handler;
use crate::join_balancer_config_handler;
use crate::message::Message;
use crate::running_balancer_handler;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;
use crate::screen::AgentRunning;
use crate::screen::Screen;
use crate::start_balancer_config_handler;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;
use crate::ui::view_agent_running::view_agent_running;
use crate::ui::view_home::view_home;
use crate::ui::view_join_balancer_config::view_join_balancer_config;
use crate::ui::view_running_balancer::view_running_balancer;
use crate::ui::view_start_balancer_config::view_start_balancer_config;

static BETA_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(include_bytes!("../../resources/images/beta.png").as_slice())
});

fn shutdown_signal_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async move |mut output| {
        if let Err(error) = wait_for_shutdown_signal().await {
            log::error!("shutdown signal listener failed: {error}");

            return;
        }

        let _ = output.send(Message::Quit).await;
    })
}

pub struct App {
    agent_cancel: Option<CancellationToken>,
    shutdown: CancellationToken,
    balancer_cancel: Option<CancellationToken>,
    screen: CurrentScreen,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            agent_cancel: None,
            shutdown: CancellationToken::new(),
            balancer_cancel: None,
            screen: CurrentScreen::default(),
        };

        (app, Task::done(Message::IcedEventLoopReady))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let screen = mem::take(&mut self.screen);

        match (screen, message) {
            (screen, Message::IcedEventLoopReady) => {
                log::info!("paddler_gui: iced event loop ready");
                self.screen = screen;

                Task::none()
            }
            (_, Message::Quit) => {
                self.shutdown.cancel();
                self.balancer_cancel = None;
                self.agent_cancel = None;

                iced::exit()
            }
            (CurrentScreen::Home(home), Message::Home(msg)) => {
                let action = HomeData::update(msg);

                match action {
                    home_handler::Action::StartBalancer => {
                        self.screen = CurrentScreen::StartBalancerConfig(home.start_balancer());

                        Task::none()
                    }
                    home_handler::Action::JoinBalancer => {
                        self.screen = CurrentScreen::JoinBalancerConfig(home.join_balancer());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::JoinBalancerConfig(mut config), Message::JoinBalancerConfig(msg)) => {
                let action = config.state_data.update(msg);

                match action {
                    join_balancer_config_handler::Action::None => {
                        self.screen = CurrentScreen::JoinBalancerConfig(config);

                        Task::none()
                    }
                    join_balancer_config_handler::Action::Cancel => {
                        self.screen = CurrentScreen::Home(config.cancel());

                        Task::none()
                    }
                    join_balancer_config_handler::Action::ConnectAgent {
                        agent_name,
                        management_address,
                        slots,
                    } => self.spawn_agent(config.connect(), agent_name, management_address, slots),
                }
            }
            (CurrentScreen::StartBalancerConfig(mut config), Message::StartBalancerConfig(msg)) => {
                let action = config.state_data.update(msg);

                match action {
                    start_balancer_config_handler::Action::None => {
                        self.screen = CurrentScreen::StartBalancerConfig(config);

                        Task::none()
                    }
                    start_balancer_config_handler::Action::Cancel => {
                        if let Some(cancel) = self.balancer_cancel.as_ref() {
                            cancel.cancel();
                        }
                        self.screen = CurrentScreen::Home(config.cancel());

                        Task::none()
                    }
                    start_balancer_config_handler::Action::StartBalancer {
                        management_addr,
                        inference_addr,
                        desired_state,
                    } => {
                        self.screen = CurrentScreen::StartBalancerConfig(config);

                        self.spawn_balancer(management_addr, inference_addr, &desired_state)
                    }
                }
            }
            (CurrentScreen::StartBalancerConfig(config), Message::BalancerStarted) => {
                self.screen = CurrentScreen::RunningBalancer(config.balancer_started());

                Task::none()
            }
            (CurrentScreen::StartBalancerConfig(config), Message::BalancerFailed(error)) => {
                log::error!("Balancer failed to start: {error}");
                self.balancer_cancel = None;
                self.screen = CurrentScreen::Home(config.balancer_failed(error));

                Task::none()
            }
            (CurrentScreen::RunningBalancer(mut running), Message::RunningBalancer(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    running_balancer_handler::Action::None => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        Task::none()
                    }
                    running_balancer_handler::Action::Stop => {
                        if let Some(cancel) = self.balancer_cancel.as_ref() {
                            cancel.cancel();
                        }
                        self.screen = CurrentScreen::RunningBalancer(running);

                        Task::none()
                    }
                    running_balancer_handler::Action::CopyToClipboard(content) => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        iced::clipboard::write::<Message>(content).discard()
                    }
                }
            }
            (CurrentScreen::RunningBalancer(running), Message::BalancerStopped) => {
                self.balancer_cancel = None;
                self.screen = CurrentScreen::Home(running.balancer_stopped());

                Task::none()
            }
            (CurrentScreen::RunningBalancer(running), Message::BalancerFailed(error)) => {
                log::error!("Balancer failed unexpectedly: {error}");
                self.balancer_cancel = None;
                self.screen = CurrentScreen::Home(running.balancer_failed(error));

                Task::none()
            }
            (CurrentScreen::AgentRunning(mut running), Message::AgentRunning(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    agent_running_handler::Action::None => {
                        self.screen = CurrentScreen::AgentRunning(running);

                        Task::none()
                    }
                    agent_running_handler::Action::Disconnect => {
                        if let Some(cancel) = self.agent_cancel.as_ref() {
                            cancel.cancel();
                        }
                        self.screen = CurrentScreen::Home(running.disconnect());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::AgentRunning(running), Message::AgentStopped) => {
                log::info!("Agent stopped");
                self.agent_cancel = None;
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentFailed(error)) => {
                log::error!("Agent failed: {error}");
                self.agent_cancel = None;
                self.screen = CurrentScreen::Home(running.agent_failed(error));

                Task::none()
            }
            (screen, Message::TabPressed { shift }) => {
                self.screen = screen;

                if shift {
                    operation::focus_previous()
                } else {
                    operation::focus_next()
                }
            }
            (screen, message) => {
                log::warn!("Unhandled message {message:?} for current screen");
                self.screen = screen;

                Task::none()
            }
        }
    }

    #[expect(
        clippy::unused_self,
        reason = "signature required by iced application API"
    )]
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Tab),
                    modifiers,
                    ..
                } => Some(Message::TabPressed {
                    shift: modifiers.shift(),
                }),
                _ => None,
            }),
            window::close_requests().map(|_| Message::Quit),
            Subscription::run(shutdown_signal_stream),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => {
                view_agent_running(&screen.state_data).map(Message::AgentRunning)
            }
            CurrentScreen::Home(screen) => view_home(&screen.state_data).map(Message::Home),
            CurrentScreen::JoinBalancerConfig(screen) => {
                view_join_balancer_config(&screen.state_data).map(Message::JoinBalancerConfig)
            }
            CurrentScreen::StartBalancerConfig(screen) => {
                view_start_balancer_config(&screen.state_data).map(Message::StartBalancerConfig)
            }
            CurrentScreen::RunningBalancer(screen) => {
                view_running_balancer(&screen.state_data).map(Message::RunningBalancer)
            }
        };

        let content_column = column![screen_content]
            .max_width(700)
            .padding([SPACING_2X * 2.0, SPACING_BASE])
            .spacing(SPACING_BASE)
            .align_x(Center);

        let base_view = container(content_column).center_x(Fill).height(Fill);

        if matches!(self.screen, CurrentScreen::Home(_)) {
            let beta_image = image(BETA_IMAGE.clone()).width(100).height(100);

            let beta_overlay = container(beta_image)
                .width(Fill)
                .height(Fill)
                .align_x(Right)
                .align_y(Bottom);

            stack![base_view, beta_overlay].into()
        } else {
            base_view.into()
        }
    }

    fn spawn_agent(
        &mut self,
        screen: Screen<AgentRunning>,
        agent_name: Option<String>,
        management_address: String,
        slots: i32,
    ) -> Task<Message> {
        let cancel = self.shutdown.child_token();
        self.agent_cancel = Some(cancel.clone());
        self.screen = CurrentScreen::AgentRunning(screen);

        Task::stream(iced::stream::channel(1, async move |mut output| {
            let mut runner = AgentRunner::start(AgentRunnerParams {
                agent_name,
                management_address,
                parent_shutdown: Some(cancel),
                slots,
            });

            let slot_aggregated_status = runner.slot_aggregated_status.clone();
            let completion_future = runner.wait_for_completion();
            tokio::pin!(completion_future);

            loop {
                match slot_aggregated_status.make_snapshot() {
                    Ok(snapshot) => {
                        if output
                            .send(Message::AgentRunning(
                                agent_running_handler::Message::AgentStatusUpdated(snapshot),
                            ))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        log::error!("Failed to make agent status snapshot: {error}");

                        return;
                    }
                }

                tokio::select! {
                    () = slot_aggregated_status.update_notifier.notified() => {}
                    result = &mut completion_future => {
                        let message = match result {
                            Ok(()) => Message::AgentStopped,
                            Err(error) => Message::AgentFailed(error.to_string()),
                        };
                        let _ = output.send(message).await;

                        return;
                    }
                }
            }
        }))
    }

    #[cfg(test)]
    pub fn shutdown_token_for_test(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    fn spawn_balancer(
        &mut self,
        management_addr: SocketAddr,
        inference_addr: SocketAddr,
        desired_state: &BalancerDesiredState,
    ) -> Task<Message> {
        let cancel = self.shutdown.child_token();
        self.balancer_cancel = Some(cancel.clone());

        let params = BalancerRunnerParams {
            buffered_request_timeout: Duration::from_secs(10),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: inference_addr,
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: management_addr,
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests: 30,
            openai_service_configuration: None,
            parent_shutdown: Some(cancel),
            state_database_type: StateDatabaseType::Memory(Box::new(desired_state.clone())),
            statsd_prefix: "paddler_".to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        };

        Task::stream(iced::stream::channel(1, async move |mut output| {
            let mut runner = match BalancerRunner::start(params).await {
                Ok(runner) => runner,
                Err(error) => {
                    let _ = output
                        .send(Message::BalancerFailed(error.to_string()))
                        .await;

                    return;
                }
            };

            let completion_future = runner.wait_for_completion();
            tokio::pin!(completion_future);

            if output.send(Message::BalancerStarted).await.is_err() {
                return;
            }

            let mut desired_state_rx = runner.balancer_desired_state_tx.subscribe();
            let mut current_desired_state = runner.initial_desired_state.clone();

            loop {
                match RunningBalancerSnapshot::build(
                    &runner.agent_controller_pool,
                    &runner.balancer_applicable_state_holder,
                    current_desired_state.clone(),
                ) {
                    Ok(snapshot) => {
                        if output
                            .send(Message::RunningBalancer(
                                running_balancer_handler::Message::SnapshotUpdated(Box::new(
                                    snapshot,
                                )),
                            ))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        log::error!("Failed to build running balancer snapshot: {error}");

                        return;
                    }
                }

                tokio::select! {
                    () = runner.agent_controller_pool.update_notifier.notified() => {}
                    () = runner.balancer_applicable_state_holder.update_notifier.notified() => {}
                    desired_state_result = desired_state_rx.recv() => {
                        match desired_state_result {
                            Ok(new_desired_state) => {
                                current_desired_state = new_desired_state;
                            }
                            Err(broadcast::error::RecvError::Lagged(missed)) => {
                                log::warn!(
                                    "Desired-state broadcast lagged by {missed} messages; \
                                     continuing with the last known state"
                                );
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                log::info!(
                                    "Desired-state broadcast closed; ending snapshot stream"
                                );

                                return;
                            }
                        }
                    }
                    result = &mut completion_future => {
                        let message = match result {
                            Ok(()) => Message::BalancerStopped,
                            Err(error) => Message::BalancerFailed(error.to_string()),
                        };
                        let _ = output.send(message).await;

                        return;
                    }
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_message_cancels_shutdown_token() {
        let (mut app, _initial_task) = App::new();
        let shutdown = app.shutdown_token_for_test();

        assert!(!shutdown.is_cancelled());

        let _exit_task = app.update(Message::Quit);

        assert!(shutdown.is_cancelled());
    }

    #[test]
    fn quit_message_drops_both_runners() {
        let (mut app, _initial_task) = App::new();

        let _exit_task = app.update(Message::Quit);

        assert!(app.agent_cancel.is_none());
        assert!(app.balancer_cancel.is_none());
    }
}
