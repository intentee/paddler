use anyhow::Result;

pub trait Validates<TOutput> {
    fn validate(self) -> Result<TOutput>;
}
