use anyhow::Context as _;
use anyhow::Result;

pub fn rms_norm(embedding: &mut [f32], eps: f32) -> Result<()> {
    let embedding_length = u16::try_from(embedding.len())
        .context("embedding length exceeds the supported maximum for normalization")?;

    let mean_square = embedding
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        / f32::from(embedding_length);

    let rms = (mean_square + eps).sqrt();

    if rms == 0.0 {
        embedding.fill(0.0);

        return Ok(());
    }

    for value in embedding.iter_mut() {
        *value /= rms;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::rms_norm;

    #[test]
    fn test_rms_norm_uniform_values() {
        let mut embedding = vec![2.0, 2.0, 2.0, 2.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        for val in &embedding {
            assert!((val - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_rms_norm_mixed_values() {
        let mut embedding = vec![1.0, 3.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        let expected_rms = 5.0_f32.sqrt();

        assert!((embedding[0] - 1.0 / expected_rms).abs() < 1e-6);
        assert!((embedding[1] - 3.0 / expected_rms).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_zero_epsilon() {
        let mut embedding = vec![0.0, 0.0, 0.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        assert_eq!(embedding, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_rms_norm_zero_vector_with_nonzero_epsilon() {
        let mut embedding = vec![0.0, 0.0];
        rms_norm(&mut embedding, 1e-6).unwrap();

        for val in &embedding {
            assert!(val.abs() < 1e-3);
        }
    }

    #[test]
    fn test_rms_norm_epsilon_prevents_division_instability() {
        let mut without_eps = vec![1e-10, 1e-10];
        let mut with_eps = without_eps.clone();
        rms_norm(&mut without_eps, 0.0).unwrap();
        rms_norm(&mut with_eps, 1e-6).unwrap();

        assert!(with_eps[0].abs() < without_eps[0].abs());
    }

    #[test]
    fn test_rms_norm_single_element() {
        let mut embedding = vec![5.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        assert!((embedding[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rms_norm_empty_embedding() {
        let mut embedding: Vec<f32> = Vec::new();
        rms_norm(&mut embedding, 0.0).unwrap();

        assert!(embedding.is_empty());
    }

    #[test]
    fn test_rms_norm_length_exceeding_u16_max_returns_error() {
        let mut embedding = vec![1.0_f32; usize::from(u16::MAX) + 1];
        let result = rms_norm(&mut embedding, 0.0);

        assert!(result.is_err());
    }

    #[test]
    fn test_rms_norm_negative_values() {
        let mut embedding = vec![-3.0, 4.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        let expected_rms = 12.5_f32.sqrt();

        assert!((embedding[0] - (-3.0 / expected_rms)).abs() < 1e-6);
        assert!((embedding[1] - (4.0 / expected_rms)).abs() < 1e-6);
    }
}
