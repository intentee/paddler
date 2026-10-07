use crate::runner_failure::RunnerFailure;

pub enum HomeData {
    ReturnedAfterFailure(RunnerFailure),
    Welcome,
}
