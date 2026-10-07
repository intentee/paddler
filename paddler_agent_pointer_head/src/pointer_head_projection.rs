pub struct PointerHeadProjection {
    pub bias: Vec<f32>,
    pub hidden_size: usize,
    pub weight: Vec<f32>,
}

impl PointerHeadProjection {
    #[must_use]
    pub fn project(&self, hidden_state: &[f32]) -> Vec<f32> {
        self.weight
            .chunks_exact(self.hidden_size)
            .zip(&self.bias)
            .map(|(weight_row, bias)| {
                weight_row
                    .iter()
                    .zip(hidden_state)
                    .map(|(weight, hidden_value)| weight * hidden_value)
                    .sum::<f32>()
                    + bias
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::PointerHeadProjection;

    #[test]
    fn projects_a_hidden_state_through_every_weight_row_and_adds_its_bias() {
        let projection = PointerHeadProjection {
            bias: vec![0.5, -1.0],
            hidden_size: 3,
            weight: vec![1.0, 2.0, 3.0, -1.0, 0.0, 1.0],
        };

        assert_eq!(projection.project(&[1.0, 1.0, 2.0]), vec![9.5, 0.0]);
    }
}
