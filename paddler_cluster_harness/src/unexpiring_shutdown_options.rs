use trzcina::ServiceShutdownOptions;

use crate::constants::LONGER_THAN_ANY_TEST_RUN;

#[must_use]
pub const fn unexpiring_shutdown_options() -> ServiceShutdownOptions {
    ServiceShutdownOptions {
        abort_deadline: LONGER_THAN_ANY_TEST_RUN,
        cooperative_deadline: LONGER_THAN_ANY_TEST_RUN,
    }
}
