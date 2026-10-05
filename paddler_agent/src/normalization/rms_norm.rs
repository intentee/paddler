use std::num::TryFromIntError;

pub fn rms_norm(embedding: &mut [f32], eps: f32) -> Result<(), TryFromIntError> {
    let embedding_length = u16::try_from(embedding.len())?;

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
    fn scales_uniform_values_to_one() {
        let mut embedding = vec![2.0, 2.0, 2.0, 2.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        for val in &embedding {
            assert!((val - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn divides_mixed_values_by_their_root_mean_square() {
        let mut embedding = vec![1.0, 3.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        let expected_rms = 5.0_f32.sqrt();

        assert!((embedding[0] - 1.0 / expected_rms).abs() < 1e-6);
        assert!((embedding[1] - 3.0 / expected_rms).abs() < 1e-6);
    }

    #[test]
    fn leaves_a_zero_vector_at_zero_without_epsilon() {
        let mut embedding = vec![0.0, 0.0, 0.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        assert_eq!(embedding, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn keeps_a_zero_vector_near_zero_with_epsilon() {
        let mut embedding = vec![0.0, 0.0];
        rms_norm(&mut embedding, 1e-6).unwrap();

        for val in &embedding {
            assert!(val.abs() < 1e-3);
        }
    }

    #[test]
    fn epsilon_dampens_the_scaling_of_tiny_values() {
        let mut without_eps = vec![1e-10, 1e-10];
        let mut with_eps = without_eps.clone();
        rms_norm(&mut without_eps, 0.0).unwrap();
        rms_norm(&mut with_eps, 1e-6).unwrap();

        assert!(with_eps[0].abs() < without_eps[0].abs());
    }

    #[test]
    fn scales_a_single_element_to_one() {
        let mut embedding = vec![5.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        assert!((embedding[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn leaves_an_empty_embedding_empty() {
        let mut embedding: Vec<f32> = Vec::new();
        rms_norm(&mut embedding, 0.0).unwrap();

        assert!(embedding.is_empty());
    }

    #[test]
    fn rejects_an_embedding_longer_than_u16_max() {
        let mut embedding = vec![1.0_f32; usize::from(u16::MAX) + 1];
        let result = rms_norm(&mut embedding, 0.0);

        assert!(result.is_err());
    }

    #[test]
    fn preserves_the_sign_of_negative_values() {
        let mut embedding = vec![-3.0, 4.0];
        rms_norm(&mut embedding, 0.0).unwrap();

        let expected_rms = 12.5_f32.sqrt();

        assert!((embedding[0] - (-3.0 / expected_rms)).abs() < 1e-6);
        assert!((embedding[1] - (4.0 / expected_rms)).abs() < 1e-6);
    }
}
