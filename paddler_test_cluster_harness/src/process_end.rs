use anyhow::Error;

#[derive(Debug)]
pub enum ProcessEnd {
    Clean,
    Failed(Error),
}

impl ProcessEnd {
    pub fn failed<TFailure>(failure: TFailure) -> Self
    where
        TFailure: Into<Error>,
    {
        Self::Failed(failure.into())
    }

    pub fn into_result(self) -> Result<(), Error> {
        match self {
            Self::Clean => Ok(()),
            Self::Failed(exit_failure) => Err(exit_failure),
        }
    }
}
