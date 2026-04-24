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
