use anyhow::Context as _;
use anyhow::Result;

pub fn rms_norm(embedding: &[f32], eps: f32) -> Result<Vec<f32>> {
    if embedding.is_empty() {
        return Ok(Vec::new());
    }

    let embedding_length = u16::try_from(embedding.len())
        .context("embedding length exceeds the supported maximum for normalization")?;

    let mean_square = embedding
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        / f32::from(embedding_length);

    let rms = (mean_square + eps).sqrt();

    if rms == 0.0 {
        return Ok(vec![0.0; embedding.len()]);
    }

    Ok(embedding.iter().map(|&val| val / rms).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_norm_uniform_values() {
        let embedding = vec![2.0, 2.0, 2.0, 2.0];
        let result = rms_norm(&embedding, 0.0).unwrap();

        for val in &result {
            assert!((val - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_rms_norm_mixed_values() {
        let embedding = vec![1.0, 3.0];
        let result = rms_norm(&embedding, 0.0).unwrap();

        let expected_rms = 5.0_f32.sqrt();

        assert!((result[0] - 1.0 / expected_rms).abs() < 1e-6);
        assert!((result[1] - 3.0 / expected_rms).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_zero_epsilon() {
        let embedding = vec![0.0, 0.0, 0.0];
        let result = rms_norm(&embedding, 0.0).unwrap();

        assert_eq!(result, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_nonzero_epsilon() {
        let embedding = vec![0.0, 0.0];
        let result = rms_norm(&embedding, 1e-6).unwrap();

        for val in &result {
            assert!(val.abs() < 1e-3);
        }
    }

    #[test]
    fn test_rms_norm_epsilon_prevents_division_instability() {
        let embedding = vec![1e-10, 1e-10];
        let without_eps = rms_norm(&embedding, 0.0).unwrap();
        let with_eps = rms_norm(&embedding, 1e-6).unwrap();

        assert!(with_eps[0].abs() < without_eps[0].abs());
    }

    #[test]
    fn test_rms_norm_single_element() {
        let embedding = vec![5.0];
        let result = rms_norm(&embedding, 0.0).unwrap();

        assert!((result[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_empty_embedding() {
        let embedding: Vec<f32> = Vec::new();
        let result = rms_norm(&embedding, 0.0).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_rms_norm_length_exceeding_u16_max_returns_error() {
        let embedding = vec![1.0_f32; usize::from(u16::MAX) + 1];
        let result = rms_norm(&embedding, 0.0);

        assert!(result.is_err());
    }

    #[test]
    fn test_rms_norm_negative_values() {
        let embedding = vec![-3.0, 4.0];
        let result = rms_norm(&embedding, 0.0).unwrap();

        let expected_rms = 12.5_f32.sqrt();

        assert!((result[0] - (-3.0 / expected_rms)).abs() < 1e-6);
        assert!((result[1] - (4.0 / expected_rms)).abs() < 1e-6);
    }
}
