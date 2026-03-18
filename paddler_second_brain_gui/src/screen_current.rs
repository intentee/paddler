use crate::screen::AgentRunning;
use crate::screen::Home;
use crate::screen::JoinClusterConfig;
use crate::screen::RunningCluster;
use crate::screen::Screen;
use crate::screen::StartClusterConfig;
use crate::screen::StartingCluster;
use crate::screen::StoppingCluster;

pub enum CurrentScreen {
    AgentRunning(Screen<AgentRunning>),
    Home(Screen<Home>),
    JoinClusterConfig(Screen<JoinClusterConfig>),
    StartClusterConfig(Screen<StartClusterConfig>),
    StartingCluster(Screen<StartingCluster>),
    RunningCluster(Screen<RunningCluster>),
    StoppingCluster(Screen<StoppingCluster>),
}

impl Default for CurrentScreen {
    fn default() -> Self {
        CurrentScreen::Home(Screen::<Home>::builder().build())
    }
}
