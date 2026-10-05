use std::time::Duration;

const MODEL_LOAD: Duration = Duration::from_mins(10);
const RELEASE: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug)]
pub struct ObservationWindow {
    duration: Duration,
}

impl ObservationWindow {
    #[must_use]
    pub const fn model_load() -> Self {
        Self {
            duration: MODEL_LOAD,
        }
    }

    #[must_use]
    pub const fn release() -> Self {
        Self { duration: RELEASE }
    }

    #[must_use]
    pub const fn duration(self) -> Duration {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::ObservationWindow;

    #[test]
    fn a_release_is_observed_for_far_less_time_than_a_model_load() {
        assert!(
            ObservationWindow::release().duration() < ObservationWindow::model_load().duration()
        );
    }
}
