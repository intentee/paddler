use std::num::NonZeroU16;
use std::path::Path;

use llama_cpp_bindings::gguf_context::GgufContext;
use llama_cpp_bindings::gguf_tensor_f32::GgufTensorF32;

use crate::pointer_head_delimiters::PointerHeadDelimiters;
use crate::pointer_head_error::PointerHeadError;
use crate::pointer_head_input_normalization::PointerHeadInputNormalization;
use crate::pointer_head_projection::PointerHeadProjection;

const TEMPERATURE_KEY: &str = "pointer_head.temperature";
const TENSORS: [&str; 4] = [
    "pointer_head.key.bias",
    "pointer_head.key.weight",
    "pointer_head.query.bias",
    "pointer_head.query.weight",
];

fn read_temperature(gguf_context: &GgufContext) -> Result<f32, PointerHeadError> {
    let temperature = gguf_context
        .find_key(TEMPERATURE_KEY)
        .and_then(|key_id| gguf_context.val_f32(key_id))
        .map_err(|source| PointerHeadError::KeyUnreadable {
            key: TEMPERATURE_KEY.to_owned(),
            source,
        })?;

    if temperature.is_finite() && temperature > 0.0 {
        Ok(temperature)
    } else {
        Err(PointerHeadError::TemperatureOutOfRange { temperature })
    }
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let largest_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponentials: Vec<f32> = logits
        .iter()
        .map(|logit| (logit - largest_logit).exp())
        .collect();
    let exponential_sum: f32 = exponentials.iter().sum();

    exponentials
        .iter()
        .map(|exponential| exponential / exponential_sum)
        .collect()
}

pub struct PointerHead {
    pub delimiters: PointerHeadDelimiters,
    hidden_size: i64,
    pub input_normalization: PointerHeadInputNormalization,
    pub key_projection: PointerHeadProjection,
    pub logit_scale: f32,
    pub query_projection: PointerHeadProjection,
    pub temperature: f32,
}

impl PointerHead {
    /// # Errors
    ///
    /// Returns [`PointerHeadError::FileUnreadable`] when the file is not a readable GGUF file, [`PointerHeadError::KeyUnreadable`] or [`PointerHeadError::TemperatureOutOfRange`] when the temperature is missing or not a finite positive number, [`PointerHeadError::KeyUnreadable`] or [`PointerHeadError::InputNormalizationUnknown`] when a delimiter or the input normalization cannot be read, [`PointerHeadError::TensorUnreadable`] when a projection tensor is missing or not stored as f32, [`PointerHeadError::TensorShapeMismatch`] when the projections do not share one head and hidden dimension, [`PointerHeadError::HeadDimensionOutOfRange`] when the head dimension does not fit sixteen bits, and [`PointerHeadError::HeadDimensionZero`] when it is zero.
    pub fn load(path: &Path) -> Result<Self, PointerHeadError> {
        let gguf_context =
            GgufContext::from_file(path).map_err(|source| PointerHeadError::FileUnreadable {
                path: path.to_path_buf(),
                source,
            })?;
        let temperature = read_temperature(&gguf_context)?;
        let delimiters = PointerHeadDelimiters::load(&gguf_context)?;
        let input_normalization = PointerHeadInputNormalization::load(&gguf_context)?;
        let mut tensor_shapes = [[0; 4]; 4];
        let mut tensor_values: [Vec<f32>; 4] = Default::default();

        for ((tensor, tensor_shape), tensor_value) in TENSORS
            .iter()
            .zip(&mut tensor_shapes)
            .zip(&mut tensor_values)
        {
            let GgufTensorF32 { shape, values } =
                gguf_context.read_tensor_f32(tensor).map_err(|source| {
                    PointerHeadError::TensorUnreadable {
                        tensor: (*tensor).to_owned(),
                        source,
                    }
                })?;

            *tensor_shape = shape;
            *tensor_value = values;
        }

        let [key_bias_shape, key_weight_shape, _, _] = tensor_shapes;
        let declared_head_dim = key_bias_shape[0];
        let hidden_size = key_weight_shape[0];
        let bias_shape = [declared_head_dim, 1, 1, 1];
        let weight_shape = [hidden_size, declared_head_dim, 1, 1];

        for ((tensor, actual), expected) in TENSORS.iter().zip(tensor_shapes).zip([
            bias_shape,
            weight_shape,
            bias_shape,
            weight_shape,
        ]) {
            if actual != expected {
                return Err(PointerHeadError::TensorShapeMismatch {
                    tensor: (*tensor).to_owned(),
                    expected,
                    actual,
                });
            }
        }

        let [key_bias, key_weight, query_bias, query_weight] = tensor_values;

        let head_dim = NonZeroU16::new(u16::try_from(declared_head_dim).map_err(|source| {
            PointerHeadError::HeadDimensionOutOfRange {
                head_dim: declared_head_dim,
                source,
            }
        })?)
        .ok_or(PointerHeadError::HeadDimensionZero)?;

        Ok(Self {
            delimiters,
            hidden_size,
            input_normalization,
            key_projection: PointerHeadProjection {
                bias: key_bias,
                head_dim,
                weight: key_weight,
            },
            logit_scale: f32::from(head_dim.get()).sqrt().recip(),
            query_projection: PointerHeadProjection {
                bias: query_bias,
                head_dim,
                weight: query_weight,
            },
            temperature,
        })
    }

    #[must_use]
    pub const fn hidden_size(&self) -> i64 {
        self.hidden_size
    }

    #[must_use]
    pub fn option_probabilities(&self, output_hidden_states: &[Vec<f32>]) -> Vec<f32> {
        let Some((decide_hidden_state, option_hidden_states)) = output_hidden_states.split_last()
        else {
            return Vec::new();
        };
        let query = self.query_projection.project(decide_hidden_state);

        softmax(
            &option_hidden_states
                .iter()
                .map(|option_hidden_state| {
                    let key = self.key_projection.project(option_hidden_state);
                    let alignment: f32 = key
                        .iter()
                        .zip(&query)
                        .map(|(key_value, query_value)| key_value * query_value)
                        .sum();

                    alignment * self.logit_scale / self.temperature
                })
                .collect::<Vec<f32>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::PointerHead;
    use crate::pointer_head_delimiters::PointerHeadDelimiters;
    use crate::pointer_head_input_normalization::PointerHeadInputNormalization;
    use crate::pointer_head_projection::PointerHeadProjection;

    fn identity_projection() -> PointerHeadProjection {
        PointerHeadProjection {
            bias: vec![0.0, 0.0],
            head_dim: NonZeroU16::new(2).unwrap(),
            weight: vec![1.0, 0.0, 0.0, 1.0],
        }
    }

    fn pointer_head() -> PointerHead {
        PointerHead {
            delimiters: PointerHeadDelimiters {
                decide: "<decide>".to_owned(),
                option_end: "</option>".to_owned(),
                option_start: "<option>".to_owned(),
                question: "<question>".to_owned(),
                state: "<state>".to_owned(),
            },
            hidden_size: 2,
            input_normalization: PointerHeadInputNormalization::Unnormalized,
            key_projection: identity_projection(),
            logit_scale: 0.5,
            query_projection: identity_projection(),
            temperature: 2.0,
        }
    }

    #[test]
    fn options_aligned_with_the_decision_get_the_most_probability() {
        let [aligned_probability, misaligned_probability]: [f32; 2] = pointer_head()
            .option_probabilities(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![4.0, 0.0]])
            .try_into()
            .expect("two options must yield two probabilities");
        let expected_aligned_probability = 1.0 / (1.0 + (-1.0_f32).exp());

        assert!((aligned_probability - expected_aligned_probability).abs() < f32::EPSILON);
        assert!(
            (misaligned_probability - (1.0 - expected_aligned_probability)).abs() < f32::EPSILON
        );
    }

    #[test]
    fn a_question_without_outputs_has_no_probabilities() {
        assert_eq!(pointer_head().option_probabilities(&[]), Vec::<f32>::new());
    }
}
