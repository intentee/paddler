use crate::finished_serving_startup::FinishedServingStartup;
use crate::preparation_outcome::PreparationOutcome;

pub enum ServingStartupProgress {
    Prepared(PreparationOutcome),
    Replaced(FinishedServingStartup),
}
