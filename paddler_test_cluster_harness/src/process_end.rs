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
}
