use std::mem;
use std::sync::LazyLock;

use async_stream::stream;
use command_handler::shutdown_signal::register_shutdown_signals;
use iced::Bottom;
use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Right;
use iced::Subscription;
use iced::Task;
use iced::clipboard::write;
use iced::exit;
use iced::futures::Stream;
use iced::keyboard::Event as KeyboardEvent;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::keyboard::listen;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::operation;
use iced::widget::stack;
use iced::window;
use log::error;
use log::info;
use log::warn;
use open::that;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;

use crate::agent_runner_messages::agent_runner_messages;
use crate::agent_running_action::AgentRunningAction;
use crate::balancer_launch::BalancerLaunch;
use crate::balancer_runner_messages::balancer_runner_messages;
use crate::current_screen::CurrentScreen;
use crate::home_message::HomeMessage;
use crate::join_balancer_form_action::JoinBalancerFormAction;
use crate::message::Message;
use crate::running_balancer_action::RunningBalancerAction;
use crate::screen::AgentRunning;
use crate::screen::Screen;
use crate::start_balancer_form_action::StartBalancerFormAction;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;

static BETA_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(include_bytes!("../../resources/images/beta.png").as_slice())
});

fn shutdown_signal_stream() -> impl Stream<Item = Message> {
    stream! {
        match register_shutdown_signals() {
            Ok(shutdown_signals) => match shutdown_signals.wait().await {
                Ok(()) => yield Message::Quit,
                Err(error) => error!("shutdown signal listener failed: {error}"),
            },
            Err(error) => error!("failed to register shutdown signal handlers: {error}"),
        }
    }
}

pub struct App {
    screen: CurrentScreen,
    shutdown: CancellationToken,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            screen: CurrentScreen::default(),
            shutdown: CancellationToken::new(),
        };

        (app, Task::done(Message::IcedEventLoopReady))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let screen = mem::take(&mut self.screen);

        match (screen, message) {
            (screen, Message::IcedEventLoopReady) => {
                info!("paddler_gui: iced event loop ready");
                self.screen = screen;

                Task::none()
            }
            (_, Message::Quit) => {
                self.shutdown.cancel();

                exit()
            }
            (CurrentScreen::Home(home), Message::Home(HomeMessage::StartBalancer)) => {
                self.screen = CurrentScreen::StartBalancerForm(home.start_balancer());

                Task::none()
            }
            (CurrentScreen::Home(home), Message::Home(HomeMessage::JoinBalancer)) => {
                self.screen = CurrentScreen::JoinBalancerForm(home.join_balancer());

                Task::none()
            }
            (CurrentScreen::JoinBalancerForm(mut form), Message::JoinBalancerForm(msg)) => {
                let action = form.state_data.update(msg);

                match action {
                    JoinBalancerFormAction::None => {
                        self.screen = CurrentScreen::JoinBalancerForm(form);

                        Task::none()
                    }
                    JoinBalancerFormAction::Cancel => {
                        self.screen = CurrentScreen::Home(form.cancel());

                        Task::none()
                    }
                    JoinBalancerFormAction::ConnectAgent(agent_bootstrap_config) => self
                        .spawn_agent(
                            form.connect(self.shutdown.child_token()),
                            agent_bootstrap_config,
                        ),
                }
            }
            (CurrentScreen::StartBalancerForm(mut form), Message::StartBalancerForm(msg)) => {
                let action = form.state_data.update(msg);

                match action {
                    StartBalancerFormAction::None => {
                        self.screen = CurrentScreen::StartBalancerForm(form);

                        Task::none()
                    }
                    StartBalancerFormAction::Cancel => {
                        if let BalancerLaunch::Starting(cancellation_token) =
                            &form.state_data.launch
                        {
                            cancellation_token.cancel();
                        }
                        self.screen = CurrentScreen::Home(form.cancel());

                        Task::none()
                    }
                    StartBalancerFormAction::StartBalancer(bootstrap_config) => {
                        let cancellation_token = self.shutdown.child_token();

                        form.state_data.launch =
                            BalancerLaunch::Starting(cancellation_token.clone());
                        self.screen = CurrentScreen::StartBalancerForm(form);

                        Task::stream(balancer_runner_messages(BalancerRunnerParams {
                            bootstrap_config: *bootstrap_config,
                            cancellation_token,
                            shutdown_options: ServiceShutdownOptions::default(),
                        }))
                    }
                }
            }
            (
                CurrentScreen::StartBalancerForm(form),
                Message::BalancerStarted {
                    addresses,
                    cancellation_token,
                    snapshot,
                },
            ) => {
                self.screen = CurrentScreen::RunningBalancer(form.balancer_started(
                    addresses,
                    cancellation_token,
                    snapshot,
                ));

                Task::none()
            }
            (CurrentScreen::StartBalancerForm(form), Message::BalancerFailed(error)) => {
                error!("Balancer failed to start: {error}");
                self.screen = CurrentScreen::Home(form.balancer_failed(error));

                Task::none()
            }
            (CurrentScreen::RunningBalancer(mut running), Message::RunningBalancer(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    RunningBalancerAction::None => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        Task::none()
                    }
                    RunningBalancerAction::CopyToClipboard(content) => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        write::<Message>(content).discard()
                    }
                    RunningBalancerAction::OpenUrl(url) => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        if let Err(error) = that(&url) {
                            error!("Failed to open URL {url}: {error}");
                        }

                        Task::none()
                    }
                }
            }
            (CurrentScreen::RunningBalancer(running), Message::BalancerStopped) => {
                self.screen = CurrentScreen::Home(running.balancer_stopped());

                Task::none()
            }
            (CurrentScreen::RunningBalancer(running), Message::BalancerFailed(error)) => {
                error!("Balancer failed unexpectedly: {error}");
                self.screen = CurrentScreen::Home(running.balancer_failed(error));

                Task::none()
            }
            (CurrentScreen::AgentRunning(mut running), Message::AgentRunning(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    AgentRunningAction::None => {
                        self.screen = CurrentScreen::AgentRunning(running);

                        Task::none()
                    }
                    AgentRunningAction::Disconnect => {
                        self.screen = CurrentScreen::Home(running.disconnect());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::AgentRunning(running), Message::AgentStopped) => {
                info!("Agent stopped");
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentFailed(error)) => {
                error!("Agent failed: {error}");
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
                warn!("Unhandled message {message:?} for current screen");
                self.screen = screen;

                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            listen().filter_map(|event| match event {
                KeyboardEvent::KeyPressed {
                    key: Key::Named(Named::Tab),
                    modifiers,
                    ..
                } => Some(Message::TabPressed {
                    shift: modifiers.shift(),
                }),
                _ => None,
            }),
            window::close_requests().map(|_closed_window_id| Message::Quit),
            Subscription::run(shutdown_signal_stream),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => {
                screen.state_data.view().map(Message::AgentRunning)
            }
            CurrentScreen::Home(screen) => screen.state_data.view().map(Message::Home),
            CurrentScreen::JoinBalancerForm(screen) => {
                screen.state_data.view().map(Message::JoinBalancerForm)
            }
            CurrentScreen::StartBalancerForm(screen) => {
                screen.state_data.view().map(Message::StartBalancerForm)
            }
            CurrentScreen::RunningBalancer(screen) => {
                screen.state_data.view().map(Message::RunningBalancer)
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
        bootstrap_config: AgentBootstrapConfig,
    ) -> Task<Message> {
        let cancellation_token = screen.state_data.cancellation_token.clone();

        self.screen = CurrentScreen::AgentRunning(screen);

        Task::stream(agent_runner_messages(AgentRunnerParams {
            bootstrap_config,
            cancellation_token,
            shutdown_options: ServiceShutdownOptions::default(),
        }))
    }
}
