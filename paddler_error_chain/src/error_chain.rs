use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::iter::successors;

pub struct ErrorChain<'error> {
    pub error: &'error dyn Error,
}

impl Display for ErrorChain<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.error).and_then(|()| {
            successors(self.error.source(), |&cause| cause.source())
                .try_for_each(|cause| write!(formatter, ": {cause}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::ErrorChain;

    #[derive(Debug, Error)]
    #[error("the disk is full")]
    struct DiskFull;

    #[derive(Debug, Error)]
    #[error("unable to write the cache")]
    struct CacheUnwritable {
        #[source]
        source: DiskFull,
    }

    #[derive(Debug, Error)]
    #[error("unable to download the model")]
    struct ModelUndownloadable {
        #[source]
        source: CacheUnwritable,
    }

    #[test]
    fn renders_the_message_followed_by_each_cause() {
        assert_eq!(
            ErrorChain {
                error: &ModelUndownloadable {
                    source: CacheUnwritable { source: DiskFull },
                },
            }
            .to_string(),
            "unable to download the model: unable to write the cache: the disk is full"
        );
    }
}
