use std::future::pending;
use std::mem;

use paddler_agent_runtime::shuts_down::ShutsDown;

use crate::finished_serving_startup::FinishedServingStartup;
use crate::startup_phase::StartupPhase;

pub enum ServingStartup {
    Idle,
    Replacing(StartupPhase<FinishedServingStartup>),
}

impl ServingStartup {
    pub fn cancel(&self) {
        match self {
            Self::Idle => {}
            Self::Replacing(StartupPhase {
                cancellation_token, ..
            }) => cancellation_token.cancel(),
        }
    }

    pub async fn next_finished_startup(&mut self) -> FinishedServingStartup {
        let finished_serving_startup = match self {
            Self::Idle => pending().await,
            Self::Replacing(StartupPhase { outcome, .. }) => outcome.as_mut().await,
        };

        drop(mem::replace(self, Self::Idle));

        finished_serving_startup
    }
}

impl ShutsDown for ServingStartup {
    async fn shut_down(self) {
        self.cancel();

        match self {
            Self::Idle => {}
            Self::Replacing(StartupPhase { outcome, .. }) => {
                outcome.await.outcome.shut_down().await;
            }
        }
    }
}
