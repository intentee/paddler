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
use paddler_bootstrap::bootstrap_agent_params::BootstrapAgentParams;
use paddler_bootstrap::bootstrap_balancer_params::BootstrapBalancerParams;
use paddler_bootstrap::unix_shutdown_signal::wait_for_unix_shutdown_signal;
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

fn unix_shutdown_signal_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async move |mut output| {
        if let Err(error) = wait_for_unix_shutdown_signal().await {
            log::error!("unix shutdown signal listener failed: {error}");

            return;
        }

        let _ = output.send(Message::Quit).await;
    })
}

pub struct App {
    agent_runner: Option<AgentRunner>,
    shutdown: CancellationToken,
    balancer_runner: Option<BalancerRunner>,
    screen: CurrentScreen,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            agent_runner: None,
            shutdown: CancellationToken::new(),
            balancer_runner: None,
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
                self.balancer_runner = None;
                self.agent_runner = None;

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
                        if let Some(runner) = self.balancer_runner.as_ref() {
                            runner.cancel();
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
                self.balancer_runner = None;
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
                        if let Some(runner) = self.balancer_runner.as_ref() {
                            runner.cancel();
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
                self.balancer_runner = None;
                self.screen = CurrentScreen::Home(running.balancer_stopped());

                Task::none()
            }
            (CurrentScreen::RunningBalancer(running), Message::BalancerFailed(error)) => {
                log::error!("Balancer failed unexpectedly: {error}");
                self.balancer_runner = None;
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
                        if let Some(runner) = self.agent_runner.as_ref() {
                            runner.cancel();
                        }
                        self.screen = CurrentScreen::Home(running.disconnect());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::AgentRunning(running), Message::AgentStopped) => {
                log::info!("Agent stopped");
                self.agent_runner = None;
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentFailed(error)) => {
                log::error!("Agent failed: {error}");
                self.agent_runner = None;
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
            Subscription::run(unix_shutdown_signal_stream),
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
        let mut runner = AgentRunner::start(AgentRunnerParams {
            bootstrap_params: BootstrapAgentParams {
                agent_name,
                management_address,
                slots,
            },
            parent_shutdown: Some(self.shutdown.clone()),
        });

        let initial_status_rx = runner.take_initial_status_rx();
        let completion_rx = runner.take_completion_rx();

        self.agent_runner = Some(runner);
        self.screen = CurrentScreen::AgentRunning(screen);

        Task::batch([
            Task::perform(
                async move {
                    match completion_rx {
                        Some(rx) => rx
                            .await
                            .map_err(|error| anyhow::anyhow!("Agent runner dropped: {error}"))?,
                        None => Err(anyhow::anyhow!("Agent runner completion channel missing")),
                    }
                },
                |result: Result<(), anyhow::Error>| match result {
                    Ok(()) => Message::AgentStopped,
                    Err(error) => Message::AgentFailed(error.to_string()),
                },
            ),
            Task::stream(iced::stream::channel(1, async move |mut output| {
                let Some(initial_status_rx) = initial_status_rx else {
                    return;
                };

                let slot_aggregated_status = match initial_status_rx.await {
                    Ok(status) => status,
                    Err(error) => {
                        log::error!("Agent status channel dropped: {error}");

                        return;
                    }
                };

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

                    slot_aggregated_status.update_notifier.notified().await;
                }
            })),
        ])
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
        let mut runner = BalancerRunner::start(BalancerRunnerParams {
            bootstrap_params: BootstrapBalancerParams {
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
                state_database_type: StateDatabaseType::Memory,
                statsd_prefix: "paddler_".to_owned(),
                statsd_service_configuration: None,
                #[cfg(feature = "web_admin_panel")]
                web_admin_panel_service_configuration: None,
            },
            initial_desired_state: Some(desired_state.clone()),
            parent_shutdown: Some(self.shutdown.clone()),
        });

        let initial_bundle_rx = runner.take_initial_bundle_rx();
        let completion_rx = runner.take_completion_rx();

        self.balancer_runner = Some(runner);

        Task::batch([
            Task::perform(
                async move {
                    match completion_rx {
                        Some(rx) => rx
                            .await
                            .map_err(|error| anyhow::anyhow!("Balancer runner dropped: {error}"))?,
                        None => Err(anyhow::anyhow!("Balancer runner completion channel missing")),
                    }
                },
                |result: Result<(), anyhow::Error>| match result {
                    Ok(()) => Message::BalancerStopped,
                    Err(error) => Message::BalancerFailed(error.to_string()),
                },
            ),
            Task::stream(iced::stream::channel(1, async move |mut output| {
                let Some(initial_bundle_rx) = initial_bundle_rx else {
                    return;
                };

                let bundle = match initial_bundle_rx.await {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        log::error!("Bootstrap handoff dropped before publishing bundle: {error}");

                        return;
                    }
                };

                let mut desired_state_rx = bundle.balancer_desired_state_rx.resubscribe();
                let mut current_desired_state = bundle.initial_desired_state.clone();

                if output.send(Message::BalancerStarted).await.is_err() {
                    return;
                }

                loop {
                    match RunningBalancerSnapshot::build(
                        &bundle.agent_controller_pool,
                        &bundle.balancer_applicable_state_holder,
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
                        () = bundle.agent_controller_pool.update_notifier.notified() => {}
                        () = bundle.balancer_applicable_state_holder.update_notifier.notified() => {}
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
                    }
                }
            })),
        ])
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

        assert!(app.agent_runner.is_none());
        assert!(app.balancer_runner.is_none());
    }
}
