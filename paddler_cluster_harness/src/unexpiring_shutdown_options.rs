use trzcina::service_shutdown_options::ServiceShutdownOptions;

use crate::longer_than_any_test_run::longer_than_any_test_run;

#[must_use]
pub const fn unexpiring_shutdown_options() -> ServiceShutdownOptions {
    ServiceShutdownOptions {
        abort_deadline: longer_than_any_test_run(),
        cooperative_deadline: longer_than_any_test_run(),
    }
}
