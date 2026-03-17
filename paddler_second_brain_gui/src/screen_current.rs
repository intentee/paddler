use crate::screen::Home;
use crate::screen::RunningCluster;
use crate::screen::Screen;
use crate::screen::StartClusterConfig;
use crate::screen::StartingCluster;
use crate::screen::StoppingCluster;

pub enum CurrentScreen {
    Home(Screen<Home>),
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
