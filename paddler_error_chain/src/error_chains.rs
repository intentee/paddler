use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::error_chain::ErrorChain;

pub struct ErrorChains<'errors, TError: Error> {
    pub errors: &'errors [TError],
}

impl<TError: Error> Display for ErrorChains<'_, TError> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.errors
            .iter()
            .enumerate()
            .try_for_each(|(error_index, error)| {
                let separator = if error_index == 0 { "" } else { "; " };

                write!(formatter, "{separator}{}", ErrorChain { error })
            })
    }
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::ErrorChains;

    #[derive(Debug, Error)]
    #[error("the disk is full")]
    struct DiskFull;

    #[derive(Debug, Error)]
    enum ServiceFailure {
        #[error("the cache service stopped")]
        CacheServiceStopped {
            #[source]
            source: DiskFull,
        },
        #[error("the metrics service panicked")]
        MetricsServicePanicked,
    }

    #[test]
    fn renders_every_error_with_its_causes_one_after_another() {
        assert_eq!(
            ErrorChains {
                errors: &[
                    ServiceFailure::CacheServiceStopped { source: DiskFull },
                    ServiceFailure::MetricsServicePanicked,
                ],
            }
            .to_string(),
            "the cache service stopped: the disk is full; the metrics service panicked"
        );
    }
}
