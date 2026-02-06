pub fn l2(embedding: &[f32]) -> Vec<f32> {
    let magnitude = embedding
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        .sqrt();

    if magnitude == 0.0 {
        return vec![0.0; embedding.len()];
    }

    embedding.iter().map(|&val| val / magnitude).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_l2() {
        let embedding = vec![3.0, 4.0];
        let normalized = l2(&embedding);

        assert_eq!(normalized, vec![0.6, 0.8]);

        let zero_embedding = vec![0.0, 0.0];
        let normalized_zero = l2(&zero_embedding);

        assert_eq!(normalized_zero, vec![0.0, 0.0]);
    }
}
