use crate::screen::AgentRunning;
use crate::screen::Home;
use crate::screen::JoinClusterConfig;
use crate::screen::RunningCluster;
use crate::screen::Screen;
use crate::screen::StartClusterConfig;

pub enum CurrentScreen {
    AgentRunning(Screen<AgentRunning>),
    Home(Screen<Home>),
    JoinClusterConfig(Screen<JoinClusterConfig>),
    StartClusterConfig(Screen<StartClusterConfig>),
    RunningCluster(Screen<RunningCluster>),
}

impl Default for CurrentScreen {
    fn default() -> Self {
        use crate::home_data::HomeData;

        CurrentScreen::Home(
            Screen::<Home>::builder()
                .state_data(HomeData { error: None })
                .build(),
        )
    }
}
