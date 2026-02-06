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
