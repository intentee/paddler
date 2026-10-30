use std::future::pending;
use std::mem;

use log::error;

use paddler_agent_runtime::shuts_down::ShutsDown;

use crate::finished_serving_startup::FinishedServingStartup;
use crate::preparation_outcome::PreparationOutcome;
use crate::serving_startup_progress::ServingStartupProgress;
use crate::startup_phase::StartupPhase;

pub enum ServingStartup {
    Idle,
    Preparing(StartupPhase<PreparationOutcome>),
    Replacing(StartupPhase<FinishedServingStartup>),
}

impl ServingStartup {
    pub fn cancel(&self) {
        match self {
            Self::Idle => {}
            Self::Preparing(StartupPhase {
                cancellation_token, ..
            })
            | Self::Replacing(StartupPhase {
                cancellation_token, ..
            }) => cancellation_token.cancel(),
        }
    }

    pub async fn next_progress(&mut self) -> ServingStartupProgress {
        let serving_startup_progress = match self {
            Self::Idle => pending().await,
            Self::Preparing(StartupPhase { outcome, .. }) => {
                ServingStartupProgress::Prepared(outcome.as_mut().await)
            }
            Self::Replacing(StartupPhase { outcome, .. }) => {
                ServingStartupProgress::Replaced(outcome.as_mut().await)
            }
        };

        drop(mem::replace(self, Self::Idle));

        serving_startup_progress
    }
}

impl ShutsDown for ServingStartup {
    async fn shut_down(self) {
        self.cancel();

        match self {
            Self::Idle => {}
            Self::Preparing(StartupPhase { outcome, .. }) => match outcome.await {
                PreparationOutcome::Cancelled | PreparationOutcome::Prepared(_) => {}
                PreparationOutcome::Failed { error, .. } => {
                    error!("Failed to prepare the serving that the shutdown stopped: {error}");
                }
            },
            Self::Replacing(StartupPhase { outcome, .. }) => {
                outcome.await.outcome.shut_down().await;
            }
        }
    }
}
