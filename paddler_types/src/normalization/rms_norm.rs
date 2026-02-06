pub fn rms_norm(embedding: &[f32], eps: f32) -> Vec<f32> {
    let mean_square = embedding
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        / embedding.len() as f32;

    let rms = (mean_square + eps).sqrt();

    if rms == 0.0 {
        return vec![0.0; embedding.len()];
    }

    embedding.iter().map(|&val| val / rms).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_norm_uniform_values() {
        let embedding = vec![2.0, 2.0, 2.0, 2.0];
        let result = rms_norm(&embedding, 0.0);

        // mean_square = (4+4+4+4)/4 = 4, rms = 2.0
        // each value / 2.0 = 1.0
        for val in &result {
            assert!((val - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_rms_norm_mixed_values() {
        let embedding = vec![1.0, 3.0];
        let result = rms_norm(&embedding, 0.0);

        // mean_square = (1+9)/2 = 5, rms = sqrt(5)
        let expected_rms = 5.0_f32.sqrt();

        assert!((result[0] - 1.0 / expected_rms).abs() < 1e-6);
        assert!((result[1] - 3.0 / expected_rms).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_zero_epsilon() {
        let embedding = vec![0.0, 0.0, 0.0];
        let result = rms_norm(&embedding, 0.0);

        assert_eq!(result, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_nonzero_epsilon() {
        let embedding = vec![0.0, 0.0];
        let result = rms_norm(&embedding, 1e-6);

        // mean_square = 0, rms = sqrt(1e-6), so values = 0 / rms = 0
        for val in &result {
            assert!(val.abs() < 1e-3);
        }
    }

    #[test]
    fn test_rms_norm_epsilon_prevents_division_instability() {
        let embedding = vec![1e-10, 1e-10];
        let without_eps = rms_norm(&embedding, 0.0);
        let with_eps = rms_norm(&embedding, 1e-6);

        // With epsilon, the denominator is larger, so normalized values are smaller
        assert!(with_eps[0].abs() < without_eps[0].abs());
    }

    #[test]
    fn test_rms_norm_single_element() {
        let embedding = vec![5.0];
        let result = rms_norm(&embedding, 0.0);

        // mean_square = 25/1 = 25, rms = 5.0, result = 5/5 = 1.0
        assert!((result[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_negative_values() {
        let embedding = vec![-3.0, 4.0];
        let result = rms_norm(&embedding, 0.0);

        // mean_square = (9+16)/2 = 12.5, rms = sqrt(12.5)
        let expected_rms = 12.5_f32.sqrt();

        assert!((result[0] - (-3.0 / expected_rms)).abs() < 1e-6);
        assert!((result[1] - (4.0 / expected_rms)).abs() < 1e-6);
    }
}
