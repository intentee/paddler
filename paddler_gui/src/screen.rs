use std::collections::BTreeSet;

use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
use statum::machine;
use statum::state;
use statum::transition;

use crate::address_field::AddressField;
use crate::agent_running_data::AgentRunningData;
use crate::detect_network_interfaces::detect_network_interfaces;
use crate::home_data::HomeData;
use crate::join_balancer_form_data::JoinBalancerFormData;
use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;
use crate::start_balancer_form_data::StartBalancerFormData;

#[state]
pub enum ScreenState {
    AgentRunning(AgentRunningData),
    Home(HomeData),
    JoinBalancerForm(JoinBalancerFormData),
    StartBalancerForm(StartBalancerFormData),
    RunningBalancer(RunningBalancerData),
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    #[must_use]
    pub fn join_balancer(self) -> Screen<JoinBalancerForm> {
        self.transition_with(JoinBalancerFormData::default())
    }

    #[must_use]
    pub fn start_balancer(self) -> Screen<StartBalancerForm> {
        let suggested_address = detect_network_interfaces()
            .first()
            .map(|interface| interface.ip_address.to_string())
            .unwrap_or_default();

        self.transition_with(StartBalancerFormData {
            add_model_later: false,
            balancer_address: AddressField::required_from_user_input(format!(
                "{suggested_address}:8060"
            )),
            inference_address: AddressField::required_from_user_input(format!(
                "{suggested_address}:8061"
            )),
            model_error: None,
            selected_model: None,
            starting: false,
            web_admin_panel_address: AddressField::Empty,
            web_admin_panel_address_placeholder: format!("{suggested_address}:8062"),
        })
    }
}

#[transition]
impl Screen<JoinBalancerForm> {
    #[must_use]
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    #[must_use]
    pub fn connect(self) -> Screen<AgentRunning> {
        self.transition_map(|form_data: JoinBalancerFormData| {
            let name = if form_data.agent_name.is_empty() {
                None
            } else {
                Some(form_data.agent_name)
            };

            AgentRunningData {
                balancer_address: form_data.balancer_address.raw_text().to_owned(),
                connected: false,
                snapshot: AgentControllerSnapshot {
                    desired_slots_total: 0,
                    download_current: 0,
                    download_filename: None,
                    download_total: 0,
                    id: String::new(),
                    issues: BTreeSet::new(),
                    model_path: None,
                    name,
                    slots_processing: 0,
                    slots_total: 0,
                    state_application_status: AgentStateApplicationStatus::Fresh,
                    uses_chat_template_override: false,
                },
            }
        })
    }
}

#[transition]
impl Screen<AgentRunning> {
    #[must_use]
    pub fn disconnect(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    #[must_use]
    pub fn agent_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}

#[transition]
impl Screen<StartBalancerForm> {
    #[must_use]
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    #[must_use]
    pub fn balancer_started(
        self,
        balancer_address: String,
        web_admin_panel_address: Option<String>,
    ) -> Screen<RunningBalancer> {
        self.transition_map(|_form_data: StartBalancerFormData| RunningBalancerData {
            balancer_address,
            snapshot: RunningBalancerSnapshot::default(),
            stopping: false,
            web_admin_panel_address,
        })
    }

    #[must_use]
    pub fn balancer_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}

#[transition]
impl Screen<RunningBalancer> {
    #[must_use]
    pub fn balancer_stopped(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    #[must_use]
    pub fn balancer_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;

    use super::AddressField;
    use super::AgentRunning;
    use super::HomeData;
    use super::JoinBalancerForm;
    use super::JoinBalancerFormData;
    use super::RunningBalancer;
    use super::RunningBalancerData;
    use super::RunningBalancerSnapshot;
    use super::Screen;
    use super::StartBalancerForm;
    use super::StartBalancerFormData;
    use crate::connect_address_field::ConnectAddressField;

    fn home() -> Screen<super::Home> {
        Screen::<super::Home>::builder()
            .state_data(HomeData { error: None })
            .build()
    }

    fn join_form(prefilled: JoinBalancerFormData) -> Screen<JoinBalancerForm> {
        Screen::<JoinBalancerForm>::builder()
            .state_data(prefilled)
            .build()
    }

    fn start_form(prefilled: StartBalancerFormData) -> Screen<StartBalancerForm> {
        Screen::<StartBalancerForm>::builder()
            .state_data(prefilled)
            .build()
    }

    fn agent_running(prefilled: super::AgentRunningData) -> Screen<AgentRunning> {
        Screen::<AgentRunning>::builder()
            .state_data(prefilled)
            .build()
    }

    fn running(prefilled: RunningBalancerData) -> Screen<RunningBalancer> {
        Screen::<RunningBalancer>::builder()
            .state_data(prefilled)
            .build()
    }

    fn fresh_start_form_data() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: AddressField::Invalid {
                raw: "127.0.0.1:8060".to_owned(),
                error: "placeholder".to_owned(),
            },
            inference_address: AddressField::Invalid {
                raw: "127.0.0.1:8061".to_owned(),
                error: "placeholder".to_owned(),
            },
            model_error: None,
            selected_model: None,
            starting: true,
            web_admin_panel_address: AddressField::Empty,
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

    fn fresh_agent_running_data() -> super::AgentRunningData {
        use std::collections::BTreeSet;

        use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;

        super::AgentRunningData {
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

    #[test]
    fn home_to_join_balancer_form_starts_with_empty_form_state() -> Result<()> {
        let _moved = home().join_balancer();
        Ok(())
    }

    #[test]
    fn home_to_start_balancer_form_seeds_addresses_from_detected_interfaces() -> Result<()> {
        let next = home().start_balancer();

        assert!(
            !next.state_data.balancer_address.raw_text().is_empty(),
            "expected start form to be seeded with a balancer_address"
        );
        assert!(
            next.state_data
                .balancer_address
                .raw_text()
                .ends_with(":8060"),
            "expected default balancer port suffix :8060, got {}",
            next.state_data.balancer_address.raw_text()
        );

        Ok(())
    }

    #[test]
    fn join_balancer_form_cancel_returns_home_without_error() -> Result<()> {
        let next = join_form(JoinBalancerFormData::default()).cancel();

        assert!(
            next.state_data.error.is_none(),
            "cancel should not surface an error on home"
        );
        Ok(())
    }

    #[test]
    fn join_balancer_form_connect_with_empty_agent_name_sets_name_none_on_agent_screen()
    -> Result<()> {
        let form = join_form(JoinBalancerFormData {
            balancer_address: ConnectAddressField::Invalid {
                raw: "127.0.0.1:8060".to_owned(),
                error: "placeholder".to_owned(),
            },
            ..JoinBalancerFormData::default()
        });

        let next = form.connect();

        assert!(
            next.state_data.snapshot.name.is_none(),
            "expected agent name None when form field empty"
        );
        Ok(())
    }

    #[test]
    fn join_balancer_form_connect_with_filled_agent_name_sets_some_name_on_agent_screen()
    -> Result<()> {
        let form = join_form(JoinBalancerFormData {
            agent_name: "primary".to_owned(),
            balancer_address: ConnectAddressField::Invalid {
                raw: "127.0.0.1:8060".to_owned(),
                error: "placeholder".to_owned(),
            },
            ..JoinBalancerFormData::default()
        });

        let next = form.connect();

        assert_eq!(
            next.state_data.snapshot.name.as_deref(),
            Some("primary"),
            "expected agent name Some(\"primary\")"
        );
        Ok(())
    }

    #[test]
    fn agent_running_disconnect_returns_home_without_error() -> Result<()> {
        let next = agent_running(fresh_agent_running_data()).disconnect();
        assert!(
            next.state_data.error.is_none(),
            "disconnect should not surface an error on home"
        );
        Ok(())
    }

    #[test]
    fn agent_running_agent_failed_returns_home_with_error_message() -> Result<()> {
        let next = agent_running(fresh_agent_running_data()).agent_failed("oops".to_owned());

        assert_eq!(next.state_data.error.as_deref(), Some("oops"));

        Ok(())
    }

    #[test]
    fn start_balancer_form_cancel_returns_home_without_error() -> Result<()> {
        let next = start_form(fresh_start_form_data()).cancel();
        assert!(
            next.state_data.error.is_none(),
            "cancel should not surface an error on home"
        );
        Ok(())
    }

    #[test]
    fn start_balancer_form_balancer_started_carries_addresses_into_running_balancer_data()
    -> Result<()> {
        let form = start_form(fresh_start_form_data());

        let next = form.balancer_started(
            "127.0.0.1:8060".to_owned(),
            Some("127.0.0.1:8062".to_owned()),
        );

        assert_eq!(next.state_data.balancer_address, "127.0.0.1:8060");
        assert_eq!(
            next.state_data.web_admin_panel_address.as_deref(),
            Some("127.0.0.1:8062")
        );

        Ok(())
    }

    #[test]
    fn start_balancer_form_balancer_started_with_no_web_admin_address_resolves_to_none()
    -> Result<()> {
        let form = start_form(fresh_start_form_data());

        let next = form.balancer_started("127.0.0.1:8060".to_owned(), None);

        assert!(
            next.state_data.web_admin_panel_address.is_none(),
            "expected None web_admin_panel_address to remain None"
        );
        Ok(())
    }

    #[test]
    fn start_balancer_form_balancer_failed_returns_home_with_error_message() -> Result<()> {
        let next = start_form(fresh_start_form_data()).balancer_failed("nope".to_owned());

        assert_eq!(next.state_data.error.as_deref(), Some("nope"));

        Ok(())
    }

    #[test]
    fn running_balancer_balancer_stopped_returns_home_without_error() -> Result<()> {
        let next = running(fresh_running_data()).balancer_stopped();
        assert!(
            next.state_data.error.is_none(),
            "balancer_stopped should not surface an error on home"
        );
        Ok(())
    }

    #[test]
    fn running_balancer_balancer_failed_returns_home_with_error_message() -> Result<()> {
        let next = running(fresh_running_data()).balancer_failed("kaboom".to_owned());

        assert_eq!(next.state_data.error.as_deref(), Some("kaboom"));

        Ok(())
    }

    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
}
