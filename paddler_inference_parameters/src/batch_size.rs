use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

use crate::invalid_inference_parameters::InvalidInferenceParameters;

const MAX_BATCH_SIZE: u32 = i32::MAX.unsigned_abs();

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BatchSize(NonZeroU32);

impl BatchSize {
    pub const DEFAULT: Self = Self(NonZeroU32::new(2048).unwrap());

    #[must_use]
    pub const fn tokens(self) -> NonZeroU32 {
        self.0
    }

    #[must_use]
    pub const fn tokens_i32(self) -> i32 {
        self.0.get().cast_signed()
    }

    #[must_use]
    pub const fn tokens_usize(self) -> usize {
        self.0.get() as usize
    }
}

impl TryFrom<u32> for BatchSize {
    type Error = InvalidInferenceParameters;

    fn try_from(n_batch: u32) -> Result<Self, Self::Error> {
        if n_batch > MAX_BATCH_SIZE {
            return Err(InvalidInferenceParameters::BatchSizeTooLarge {
                n_batch,
                max_batch_size: MAX_BATCH_SIZE,
            });
        }

        NonZeroU32::new(n_batch)
            .map(Self)
            .ok_or(InvalidInferenceParameters::BatchSizeZero)
    }
}

impl From<BatchSize> for u32 {
    fn from(batch_size: BatchSize) -> Self {
        batch_size.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::BatchSize;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;

    #[test]
    fn rejects_a_zero_batch() {
        assert_eq!(
            BatchSize::try_from(0),
            Err(InvalidInferenceParameters::BatchSizeZero)
        );
    }

    #[test]
    fn rejects_a_batch_llama_cpp_cannot_address() {
        assert_eq!(
            BatchSize::try_from(u32::MAX),
            Err(InvalidInferenceParameters::BatchSizeTooLarge {
                n_batch: u32::MAX,
                max_batch_size: i32::MAX.unsigned_abs(),
            })
        );
    }

    #[test]
    fn the_largest_batch_keeps_its_value_in_every_width() {
        let batch_size = BatchSize::try_from(i32::MAX.unsigned_abs()).unwrap();

        assert_eq!(batch_size.tokens_i32(), i32::MAX);
        assert_eq!(batch_size.tokens_usize(), i32::MAX.unsigned_abs() as usize);
        assert_eq!(u32::from(batch_size), batch_size.tokens().get());
    }
}
