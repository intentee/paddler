use crate::result_step::ResultStep;

pub trait StreamableResult: Sized {
    type Final: Into<Self>;
    type Partial: Into<Self>;

    fn into_step(self) -> ResultStep<Self::Partial, Self::Final>;
}
