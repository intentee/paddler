use crate::screen::AgentRunning;
use crate::screen::Home;
use crate::screen::JoinBalancerConfig;
use crate::screen::RunningBalancer;
use crate::screen::Screen;
use crate::screen::StartBalancerConfig;

pub enum CurrentScreen {
    AgentRunning(Screen<AgentRunning>),
    Home(Screen<Home>),
    JoinBalancerConfig(Screen<JoinBalancerConfig>),
    StartBalancerConfig(Screen<StartBalancerConfig>),
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
