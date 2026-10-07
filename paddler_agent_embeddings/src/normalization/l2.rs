pub fn l2(embedding: &mut [f32]) {
    let magnitude = embedding
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        .sqrt();

    if magnitude == 0.0 {
        embedding.fill(0.0);

        return;
    }

    for value in embedding.iter_mut() {
        *value /= magnitude;
    }
}

#[cfg(test)]
mod tests {
    use super::l2;

    #[test]
    fn scales_an_embedding_to_unit_length() {
        let mut embedding = vec![3.0, 4.0];
        l2(&mut embedding);

        assert_eq!(embedding, vec![0.6, 0.8]);
    }

    #[test]
    fn leaves_a_zero_embedding_at_zero() {
        let mut zero_embedding = vec![0.0, 0.0];
        l2(&mut zero_embedding);

        assert_eq!(zero_embedding, vec![0.0, 0.0]);
    }
}
