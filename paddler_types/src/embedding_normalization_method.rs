use std::mem;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingNormalizationMethod {
    L2,
    None,
    RmsNorm { epsilon: f32 },
}

impl EmbeddingNormalizationMethod {
    pub fn can_transform_to(&self, _other: &EmbeddingNormalizationMethod) -> bool {
        if matches!(self, EmbeddingNormalizationMethod::None) {
            return true;
        }

        false
    }

    pub fn needs_transformation_to(&self, other: &EmbeddingNormalizationMethod) -> bool {
        mem::discriminant(self) != mem::discriminant(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_transform_from_none_to_l2() {
        let source = EmbeddingNormalizationMethod::None;

        assert!(source.can_transform_to(&EmbeddingNormalizationMethod::L2));
    }

    #[test]
    fn test_can_transform_from_none_to_rms_norm() {
        let source = EmbeddingNormalizationMethod::None;

        assert!(source.can_transform_to(&EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 }));
    }

    #[test]
    fn test_can_transform_from_none_to_none() {
        let source = EmbeddingNormalizationMethod::None;

        assert!(source.can_transform_to(&EmbeddingNormalizationMethod::None));
    }

    #[test]
    fn test_cannot_transform_from_l2_to_anything() {
        let source = EmbeddingNormalizationMethod::L2;

        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::None));
        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::L2));
        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 }));
    }

    #[test]
    fn test_cannot_transform_from_rms_norm_to_anything() {
        let source = EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 };

        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::None));
        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::L2));
        assert!(!source.can_transform_to(&EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 }));
    }

    #[test]
    fn test_needs_transformation_between_different_methods() {
        let none = EmbeddingNormalizationMethod::None;
        let l2 = EmbeddingNormalizationMethod::L2;
        let rms = EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 };

        assert!(none.needs_transformation_to(&l2));
        assert!(none.needs_transformation_to(&rms));
        assert!(l2.needs_transformation_to(&none));
        assert!(l2.needs_transformation_to(&rms));
        assert!(rms.needs_transformation_to(&none));
        assert!(rms.needs_transformation_to(&l2));
    }

    #[test]
    fn test_no_transformation_needed_for_same_method() {
        assert!(
            !EmbeddingNormalizationMethod::None
                .needs_transformation_to(&EmbeddingNormalizationMethod::None)
        );
        assert!(
            !EmbeddingNormalizationMethod::L2
                .needs_transformation_to(&EmbeddingNormalizationMethod::L2)
        );
    }

    #[test]
    fn test_no_transformation_needed_for_rms_norm_with_different_epsilon() {
        let rms_a = EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 };
        let rms_b = EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-3 };

        // Same discriminant regardless of epsilon value
        assert!(!rms_a.needs_transformation_to(&rms_b));
    }
}
