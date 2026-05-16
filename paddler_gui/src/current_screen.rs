use crate::screen::AgentRunning;
use crate::screen::Home;
use crate::screen::JoinBalancerForm;
use crate::screen::RunningBalancer;
use crate::screen::Screen;
use crate::screen::StartBalancerForm;

pub enum CurrentScreen {
    AgentRunning(Screen<AgentRunning>),
    Home(Screen<Home>),
    JoinBalancerForm(Screen<JoinBalancerForm>),
    StartBalancerForm(Screen<StartBalancerForm>),
    RunningBalancer(Screen<RunningBalancer>),
}

impl Default for CurrentScreen {
    fn default() -> Self {
        use crate::home_data::HomeData;

        Self::Home(
            Screen::<Home>::builder()
                .state_data(HomeData { error: None })
                .build(),
        )
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;

    use super::CurrentScreen;

    #[test]
    fn default_current_screen_is_home_with_no_error() -> Result<()> {
        let screen = CurrentScreen::default();

        assert!(matches!(
            screen,
            CurrentScreen::Home(home) if home.state_data.error.is_none()
        ));

        Ok(())
    }
}
