use std::mem;
use std::sync::LazyLock;
use std::time::Duration;

use iced::Bottom;
use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Right;
use iced::Subscription;
use iced::Task;
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
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::template_data::TemplateData;
#[cfg(feature = "web_admin_panel")]
use paddler::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_bootstrap::shutdown_signal::register_shutdown_signals;
use paddler_ports::bound_port::BoundPort;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio_util::sync::CancellationToken;

use crate::agent_running_handler;
use crate::current_screen::CurrentScreen;
use crate::drive_agent_stream::drive_agent_stream;
use crate::drive_balancer_stream::drive_balancer_stream;
use crate::drive_shutdown_signal_stream::drive_shutdown_signal_stream;
use crate::home_data::HomeData;
use crate::home_handler;
use crate::join_balancer_form_handler;
use crate::message::Message;
use crate::running_balancer_handler;
use crate::screen::AgentRunning;
use crate::screen::Screen;
use crate::start_balancer_form_handler;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;
use crate::ui::view_agent_running::view_agent_running;
use crate::ui::view_home::view_home;
use crate::ui::view_join_balancer_form::view_join_balancer_form;
use crate::ui::view_running_balancer::view_running_balancer;
use crate::ui::view_start_balancer_form::view_start_balancer_form;

static BETA_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(include_bytes!("../../resources/images/beta.png").as_slice())
});

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
                        self.screen = CurrentScreen::StartBalancerForm(home.start_balancer());

                        Task::none()
                    }
                    home_handler::Action::JoinBalancer => {
                        self.screen = CurrentScreen::JoinBalancerForm(home.join_balancer());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::JoinBalancerForm(mut form), Message::JoinBalancerForm(msg)) => {
                let action = form.state_data.update(msg);

                match action {
                    join_balancer_form_handler::Action::None => {
                        self.screen = CurrentScreen::JoinBalancerForm(form);

                        Task::none()
                    }
                    join_balancer_form_handler::Action::Cancel => {
                        self.screen = CurrentScreen::Home(form.cancel());

                        Task::none()
                    }
                    join_balancer_form_handler::Action::ConnectAgent {
                        agent_name,
                        management_address,
                        slots,
                    } => self.spawn_agent(form.connect(), agent_name, management_address, slots),
                }
            }
            (CurrentScreen::StartBalancerForm(mut form), Message::StartBalancerForm(msg)) => {
                let action = form.state_data.update(msg);

                match action {
                    start_balancer_form_handler::Action::None => {
                        self.screen = CurrentScreen::StartBalancerForm(form);

                        Task::none()
                    }
                    start_balancer_form_handler::Action::Cancel => {
                        if let Some(cancel) = self.balancer_cancel.as_ref() {
                            cancel.cancel();
                        }
                        self.screen = CurrentScreen::Home(form.cancel());

                        Task::none()
                    }
                    start_balancer_form_handler::Action::StartBalancer {
                        management_port,
                        inference_port,
                        web_admin_panel_port,
                        desired_state,
                    } => {
                        self.screen = CurrentScreen::StartBalancerForm(form);

                        self.spawn_balancer(
                            management_port,
                            inference_port,
                            web_admin_panel_port,
                            &desired_state,
                        )
                    }
                }
            }
            (CurrentScreen::StartBalancerForm(form), Message::BalancerStarted) => {
                self.screen = CurrentScreen::RunningBalancer(form.balancer_started());

                Task::none()
            }
            (CurrentScreen::StartBalancerForm(form), Message::BalancerFailed(error)) => {
                log::error!("Balancer failed to start: {error}");
                self.balancer_cancel = None;
                self.screen = CurrentScreen::Home(form.balancer_failed(error));

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
                    running_balancer_handler::Action::OpenUrl(url) => {
                        self.screen = CurrentScreen::RunningBalancer(running);

                        if let Err(error) = open::that(&url) {
                            log::error!("Failed to open URL {url}: {error}");
                        }

                        Task::none()
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
            Subscription::run(|| {
                iced::stream::channel(1, |output| {
                    drive_shutdown_signal_stream(register_shutdown_signals(), output)
                })
            }),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => {
                view_agent_running(&screen.state_data).map(Message::AgentRunning)
            }
            CurrentScreen::Home(screen) => view_home(&screen.state_data).map(Message::Home),
            CurrentScreen::JoinBalancerForm(screen) => {
                view_join_balancer_form(&screen.state_data).map(Message::JoinBalancerForm)
            }
            CurrentScreen::StartBalancerForm(screen) => {
                view_start_balancer_form(&screen.state_data).map(Message::StartBalancerForm)
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

        let params = AgentRunnerParams {
            agent_name,
            cancellation_token: cancel,
            management_address,
            slots,
        };

        Task::stream(iced::stream::channel(1, move |output| {
            drive_agent_stream(params, output)
        }))
    }

    #[cfg(test)]
    pub fn shutdown_token_for_test(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    #[cfg(test)]
    pub fn agent_cancel_for_test(&self) -> Option<CancellationToken> {
        self.agent_cancel.clone()
    }

    #[cfg(test)]
    pub fn balancer_cancel_for_test(&self) -> Option<CancellationToken> {
        self.balancer_cancel.clone()
    }

    #[cfg(test)]
    pub fn set_balancer_cancel_for_test(&mut self, token: CancellationToken) {
        self.balancer_cancel = Some(token);
    }

    #[cfg(test)]
    pub fn set_agent_cancel_for_test(&mut self, token: CancellationToken) {
        self.agent_cancel = Some(token);
    }

    #[cfg(test)]
    pub fn current_screen_for_test(&self) -> &CurrentScreen {
        &self.screen
    }

    #[cfg(test)]
    pub fn set_screen_for_test(&mut self, screen: CurrentScreen) {
        self.screen = screen;
    }

    fn spawn_balancer(
        &mut self,
        management_port: BoundPort,
        inference_port: BoundPort,
        #[cfg_attr(
            not(feature = "web_admin_panel"),
            expect(
                unused_variables,
                reason = "web admin panel configuration is only built when the feature is enabled"
            )
        )]
        web_admin_panel_port: Option<BoundPort>,
        desired_state: &BalancerDesiredState,
    ) -> Task<Message> {
        let cancel = self.shutdown.child_token();
        self.balancer_cancel = Some(cancel.clone());

        let buffered_request_timeout = Duration::from_secs(10);
        let max_buffered_requests = 30;
        let statsd_prefix = "paddler_";

        let management_addr = management_port.socket_addr;
        let inference_addr = inference_port.socket_addr;

        #[cfg(feature = "web_admin_panel")]
        let (web_admin_panel_service_configuration, web_admin_panel_listener) =
            match web_admin_panel_port {
                Some(bound) => {
                    let admin_addr = bound.socket_addr;
                    let configuration = WebAdminPanelServiceConfiguration {
                        addr: admin_addr,
                        template_data: TemplateData {
                            buffered_request_timeout,
                            compat_openai_addr: None,
                            inference_addr: ResolvedSocketAddr {
                                input_addr: inference_addr.to_string(),
                                socket_addr: inference_addr,
                            },
                            management_addr: ResolvedSocketAddr {
                                input_addr: management_addr.to_string(),
                                socket_addr: management_addr,
                            },
                            max_buffered_requests,
                            statsd_addr: None,
                            statsd_prefix: statsd_prefix.to_owned(),
                            statsd_reporting_interval: Duration::from_secs(10),
                        },
                    };
                    (Some(configuration), Some(bound.listener))
                }
                None => (None, None),
            };

        let params = BalancerRunnerParams {
            buffered_request_timeout,
            inference_listener: Some(inference_port.listener),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: inference_addr,
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
            management_listener: Some(management_port.listener),
            management_service_configuration: ManagementServiceConfiguration {
                addr: management_addr,
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests,
            openai_listener: None,
            openai_service_configuration: None,
            cancellation_token: cancel,
            state_database_type: StateDatabaseType::Memory(Box::new(desired_state.clone())),
            statsd_prefix: statsd_prefix.to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_listener,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        };

        Task::stream(iced::stream::channel(1, move |output| {
            drive_balancer_stream(params, output)
        }))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::agent_running_data::AgentRunningData;
    use crate::join_balancer_form_data::JoinBalancerFormData;
    use crate::running_balancer_data::RunningBalancerData;
    use crate::running_balancer_snapshot::RunningBalancerSnapshot;
    use crate::screen::AgentRunning;
    use crate::screen::JoinBalancerForm;
    use crate::screen::RunningBalancer;
    use crate::screen::StartBalancerForm;
    use crate::start_balancer_form_data::StartBalancerFormData;

    fn fresh_join_form_data() -> JoinBalancerFormData {
        JoinBalancerFormData::default()
    }

    fn fresh_start_form_data() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: crate::address_field::AddressField::Empty,
            inference_address: crate::address_field::AddressField::Empty,
            model_error: None,
            selected_model: None,
            starting: false,
            web_admin_panel_address: crate::address_field::AddressField::Empty,
            web_admin_panel_address_placeholder: String::new(),
        }
    }

    fn fresh_running_data() -> RunningBalancerData {
        RunningBalancerData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            snapshot: RunningBalancerSnapshot::default(),
            stopping: false,
            web_admin_panel_address: None,
        }
    }

    fn fresh_agent_running_data() -> AgentRunningData {
        use std::collections::BTreeSet;

        use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
        use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

        AgentRunningData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            connected: false,
            snapshot: AgentControllerSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_total: 0,
                id: String::new(),
                issues: BTreeSet::new(),
                model_path: None,
                name: None,
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
            },
        }
    }

    fn app_with_screen(screen: CurrentScreen) -> App {
        let (mut app, _initial_task) = App::new();
        app.set_screen_for_test(screen);
        app
    }

    fn screen_join_form(form: JoinBalancerFormData) -> CurrentScreen {
        CurrentScreen::JoinBalancerForm(
            Screen::<JoinBalancerForm>::builder()
                .state_data(form)
                .build(),
        )
    }

    fn screen_start_form(form: StartBalancerFormData) -> CurrentScreen {
        CurrentScreen::StartBalancerForm(
            Screen::<StartBalancerForm>::builder()
                .state_data(form)
                .build(),
        )
    }

    fn screen_running(data: RunningBalancerData) -> CurrentScreen {
        CurrentScreen::RunningBalancer(
            Screen::<RunningBalancer>::builder()
                .state_data(data)
                .build(),
        )
    }

    fn screen_agent_running(data: AgentRunningData) -> CurrentScreen {
        CurrentScreen::AgentRunning(Screen::<AgentRunning>::builder().state_data(data).build())
    }

    fn bound_address_field() -> Result<crate::address_field::AddressField> {
        let bound = paddler_ports::bind_ephemeral_port::bind_ephemeral_port()?;
        Ok(crate::address_field::AddressField::Bound {
            raw: bound.socket_addr.to_string(),
            port: bound,
        })
    }

    fn bound_join_form_data() -> Result<JoinBalancerFormData> {
        Ok(JoinBalancerFormData {
            agent_name: String::new(),
            balancer_address: bound_address_field()?,
            slots_count: crate::slot_count_field::SlotCountField::Valid {
                raw: "2".to_owned(),
                value: 2,
            },
        })
    }

    fn assert_screen_is_home(app: &App) -> Result<()> {
        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::Home(_)
        ));
        Ok(())
    }

    #[test]
    fn quit_message_cancels_shutdown_token() -> Result<()> {
        let (mut app, _initial_task) = App::new();
        let shutdown = app.shutdown_token_for_test();

        assert!(
            !shutdown.is_cancelled(),
            "expected shutdown token to start uncancelled"
        );

        let _exit_task = app.update(Message::Quit);

        assert!(
            shutdown.is_cancelled(),
            "expected Quit to cancel shutdown token"
        );
        Ok(())
    }

    #[test]
    fn quit_message_drops_both_runners() -> Result<()> {
        let (mut app, _initial_task) = App::new();

        let _exit_task = app.update(Message::Quit);

        assert!(
            app.agent_cancel_for_test().is_none(),
            "expected Quit to drop agent_cancel"
        );
        assert!(
            app.balancer_cancel_for_test().is_none(),
            "expected Quit to drop balancer_cancel"
        );
        Ok(())
    }

    #[test]
    fn iced_event_loop_ready_preserves_current_screen() -> Result<()> {
        let (mut app, _) = App::new();
        let _ = app.update(Message::IcedEventLoopReady);
        assert_screen_is_home(&app)
    }

    #[test]
    fn home_start_balancer_message_transitions_to_start_balancer_form_screen() -> Result<()> {
        let (mut app, _) = App::new();

        let _ = app.update(Message::Home(home_handler::Message::StartBalancer));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::StartBalancerForm(_)
        ));
        Ok(())
    }

    #[test]
    fn home_join_balancer_message_transitions_to_join_balancer_form_screen() -> Result<()> {
        let (mut app, _) = App::new();

        let _ = app.update(Message::Home(home_handler::Message::JoinBalancer));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::JoinBalancerForm(_)
        ));
        Ok(())
    }

    #[test]
    fn join_form_setter_message_keeps_user_on_the_join_balancer_form() -> Result<()> {
        let mut app = app_with_screen(screen_join_form(fresh_join_form_data()));

        let _ = app.update(Message::JoinBalancerForm(
            join_balancer_form_handler::Message::SetAgentName("alice".to_owned()),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::JoinBalancerForm(_)
        ));
        Ok(())
    }

    #[test]
    fn join_form_cancel_message_returns_user_to_home() -> Result<()> {
        let mut app = app_with_screen(screen_join_form(fresh_join_form_data()));

        let _ = app.update(Message::JoinBalancerForm(
            join_balancer_form_handler::Message::Cancel,
        ));

        assert_screen_is_home(&app)
    }

    #[test]
    fn join_form_connect_action_transitions_to_agent_running_and_sets_agent_cancel_token()
    -> Result<()> {
        let mut app = app_with_screen(screen_join_form(bound_join_form_data()?));

        let _ = app.update(Message::JoinBalancerForm(
            join_balancer_form_handler::Message::Connect,
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::AgentRunning(_)
        ));
        assert!(
            app.agent_cancel_for_test().is_some(),
            "expected agent_cancel token to be set"
        );

        // Stop the spawned task immediately so the test does not leave a runner.
        if let Some(token) = app.agent_cancel_for_test() {
            token.cancel();
        }

        Ok(())
    }

    #[test]
    fn start_form_setter_message_keeps_user_on_the_start_balancer_form() -> Result<()> {
        let mut app = app_with_screen(screen_start_form(fresh_start_form_data()));

        let _ = app.update(Message::StartBalancerForm(
            start_balancer_form_handler::Message::SetBalancerAddress("127.0.0.1:0".to_owned()),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::StartBalancerForm(_)
        ));
        Ok(())
    }

    #[test]
    fn start_form_cancel_message_cancels_pending_balancer_token_and_returns_home() -> Result<()> {
        let mut app = app_with_screen(screen_start_form(fresh_start_form_data()));
        let token = CancellationToken::new();
        app.set_balancer_cancel_for_test(token.clone());

        let _ = app.update(Message::StartBalancerForm(
            start_balancer_form_handler::Message::Cancel,
        ));

        assert!(
            token.is_cancelled(),
            "expected balancer_cancel token to be cancelled"
        );

        assert_screen_is_home(&app)
    }

    #[test]
    fn start_form_confirm_action_sets_balancer_cancel_token_and_starts_spawn() -> Result<()> {
        let form = StartBalancerFormData {
            balancer_address: bound_address_field()?,
            inference_address: bound_address_field()?,
            add_model_later: true,
            ..fresh_start_form_data()
        };

        let mut app = app_with_screen(screen_start_form(form));

        let _ = app.update(Message::StartBalancerForm(
            start_balancer_form_handler::Message::Confirm,
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::StartBalancerForm(_)
        ));
        assert!(
            app.balancer_cancel_for_test().is_some(),
            "expected balancer_cancel token to be set after Confirm"
        );

        if let Some(token) = app.balancer_cancel_for_test() {
            token.cancel();
        }

        Ok(())
    }

    #[test]
    fn balancer_started_message_transitions_from_start_form_to_running_balancer() -> Result<()> {
        let mut app = app_with_screen(screen_start_form(fresh_start_form_data()));

        let _ = app.update(Message::BalancerStarted);

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::RunningBalancer(_)
        ));
        Ok(())
    }

    #[test]
    fn balancer_failed_message_during_startup_returns_user_to_home_with_error() -> Result<()> {
        let mut app = app_with_screen(screen_start_form(fresh_start_form_data()));
        app.set_balancer_cancel_for_test(CancellationToken::new());

        let _ = app.update(Message::BalancerFailed("bind error".to_owned()));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::Home(home) if home.state_data.error.as_deref() == Some("bind error")
        ));
        assert!(
            app.balancer_cancel_for_test().is_none(),
            "expected balancer_cancel to be dropped"
        );
        Ok(())
    }

    #[test]
    fn running_balancer_snapshot_update_keeps_user_on_the_running_balancer_screen() -> Result<()> {
        let mut app = app_with_screen(screen_running(fresh_running_data()));

        let _ = app.update(Message::RunningBalancer(
            running_balancer_handler::Message::SnapshotUpdated(Box::new(
                RunningBalancerSnapshot::default(),
            )),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::RunningBalancer(_)
        ));
        Ok(())
    }

    #[test]
    fn running_balancer_stop_message_cancels_token_and_keeps_user_on_screen() -> Result<()> {
        let mut app = app_with_screen(screen_running(fresh_running_data()));
        let token = CancellationToken::new();
        app.set_balancer_cancel_for_test(token.clone());

        let _ = app.update(Message::RunningBalancer(
            running_balancer_handler::Message::Stop,
        ));

        assert!(
            token.is_cancelled(),
            "expected Stop to cancel balancer_cancel token"
        );
        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::RunningBalancer(_)
        ));
        Ok(())
    }

    #[test]
    fn running_balancer_copy_to_clipboard_keeps_user_on_screen() -> Result<()> {
        let mut app = app_with_screen(screen_running(fresh_running_data()));

        let _ = app.update(Message::RunningBalancer(
            running_balancer_handler::Message::CopyToClipboard("text".to_owned()),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::RunningBalancer(_)
        ));
        Ok(())
    }

    #[test]
    fn running_balancer_open_url_with_invalid_url_logs_error_but_keeps_user_on_screen() -> Result<()>
    {
        let mut app = app_with_screen(screen_running(fresh_running_data()));

        let _ = app.update(Message::RunningBalancer(
            running_balancer_handler::Message::OpenUrl("not-a-real-scheme://broken".to_owned()),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::RunningBalancer(_)
        ));
        Ok(())
    }

    #[test]
    fn balancer_stopped_message_from_running_balancer_returns_user_to_home() -> Result<()> {
        let mut app = app_with_screen(screen_running(fresh_running_data()));
        app.set_balancer_cancel_for_test(CancellationToken::new());

        let _ = app.update(Message::BalancerStopped);

        assert!(
            app.balancer_cancel_for_test().is_none(),
            "expected balancer_cancel to be dropped"
        );

        assert_screen_is_home(&app)
    }

    #[test]
    fn balancer_failed_message_from_running_balancer_returns_user_to_home_with_error() -> Result<()>
    {
        let mut app = app_with_screen(screen_running(fresh_running_data()));

        let _ = app.update(Message::BalancerFailed("crash".to_owned()));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::Home(home) if home.state_data.error.as_deref() == Some("crash")
        ));
        Ok(())
    }

    #[test]
    fn agent_running_status_update_keeps_user_on_the_agent_running_screen() -> Result<()> {
        use std::collections::BTreeSet;

        use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
        use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

        let mut app = app_with_screen(screen_agent_running(fresh_agent_running_data()));

        let _ = app.update(Message::AgentRunning(
            agent_running_handler::Message::AgentStatusUpdated(SlotAggregatedStatusSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_total: 0,
                issues: BTreeSet::new(),
                model_path: None,
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
                version: 0,
            }),
        ));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::AgentRunning(_)
        ));
        Ok(())
    }

    #[test]
    fn agent_running_disconnect_message_cancels_token_and_returns_user_to_home() -> Result<()> {
        let mut app = app_with_screen(screen_agent_running(fresh_agent_running_data()));
        let token = CancellationToken::new();
        app.set_agent_cancel_for_test(token.clone());

        let _ = app.update(Message::AgentRunning(
            agent_running_handler::Message::Disconnect,
        ));

        assert!(
            token.is_cancelled(),
            "expected Disconnect to cancel agent_cancel token"
        );

        assert_screen_is_home(&app)
    }

    #[test]
    fn agent_stopped_message_returns_user_to_home_without_error() -> Result<()> {
        let mut app = app_with_screen(screen_agent_running(fresh_agent_running_data()));
        app.set_agent_cancel_for_test(CancellationToken::new());

        let _ = app.update(Message::AgentStopped);

        assert!(
            app.agent_cancel_for_test().is_none(),
            "expected agent_cancel to be dropped on AgentStopped"
        );

        assert_screen_is_home(&app)
    }

    #[test]
    fn agent_failed_message_returns_user_to_home_with_error() -> Result<()> {
        let mut app = app_with_screen(screen_agent_running(fresh_agent_running_data()));

        let _ = app.update(Message::AgentFailed("agent failure".to_owned()));

        assert!(matches!(
            app.current_screen_for_test(),
            CurrentScreen::Home(home) if home.state_data.error.as_deref() == Some("agent failure")
        ));
        Ok(())
    }

    #[test]
    fn tab_pressed_without_shift_focuses_the_next_widget() -> Result<()> {
        let (mut app, _) = App::new();
        let _ = app.update(Message::TabPressed { shift: false });
        // No screen change expected — just verify the call returns.
        assert_screen_is_home(&app)
    }

    #[test]
    fn tab_pressed_with_shift_focuses_the_previous_widget() -> Result<()> {
        let (mut app, _) = App::new();
        let _ = app.update(Message::TabPressed { shift: true });
        assert_screen_is_home(&app)
    }

    #[test]
    fn an_unhandled_message_for_the_current_screen_is_logged_and_keeps_the_screen() -> Result<()> {
        let (mut app, _) = App::new();
        // BalancerStarted is only meaningful from the StartBalancerForm screen.
        let _ = app.update(Message::BalancerStarted);
        assert_screen_is_home(&app)
    }

    #[test]
    fn view_with_home_screen_renders_the_beta_overlay_branch() -> Result<()> {
        let (app, _) = App::new();
        let _element = app.view();
        Ok(())
    }

    #[test]
    fn view_with_running_balancer_screen_renders_without_the_beta_overlay_branch() -> Result<()> {
        let app = app_with_screen(screen_running(fresh_running_data()));
        let _element = app.view();
        Ok(())
    }

    #[test]
    fn view_with_agent_running_screen_renders_the_agent_running_branch() -> Result<()> {
        let app = app_with_screen(screen_agent_running(fresh_agent_running_data()));
        let _element = app.view();
        Ok(())
    }

    #[test]
    fn view_with_join_balancer_form_renders_the_join_form_branch() -> Result<()> {
        let app = app_with_screen(screen_join_form(fresh_join_form_data()));
        let _element = app.view();
        Ok(())
    }

    #[test]
    fn view_with_start_balancer_form_renders_the_start_form_branch() -> Result<()> {
        let app = app_with_screen(screen_start_form(fresh_start_form_data()));
        let _element = app.view();
        Ok(())
    }

    #[test]
    fn start_balancer_with_web_admin_panel_address_builds_web_admin_configuration() -> Result<()> {
        let form = StartBalancerFormData {
            balancer_address: bound_address_field()?,
            inference_address: bound_address_field()?,
            web_admin_panel_address: bound_address_field()?,
            add_model_later: true,
            ..fresh_start_form_data()
        };

        let mut app = app_with_screen(screen_start_form(form));

        let _ = app.update(Message::StartBalancerForm(
            start_balancer_form_handler::Message::Confirm,
        ));

        let token = app.balancer_cancel_for_test();
        assert!(
            token.is_some(),
            "expected balancer_cancel token to be set after Confirm with web admin address"
        );

        if let Some(token) = token {
            token.cancel();
        }

        Ok(())
    }

    #[test]
    fn subscription_returns_a_batch_without_panicking() -> Result<()> {
        let (app, _) = App::new();
        let _subscription = app.subscription();
        Ok(())
    }
}
