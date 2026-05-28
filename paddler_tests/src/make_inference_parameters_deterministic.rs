use paddler::inference_parameters::InferenceParameters;

#[must_use]
pub const fn make_inference_parameters_deterministic(
    base: InferenceParameters,
) -> InferenceParameters {
    InferenceParameters {
        temperature: 0.0,
        top_k: 1,
        top_p: 1.0,
        min_p: 0.0,
        penalty_repeat: 1.0,
        penalty_presence: 0.0,
        penalty_frequency: 0.0,
        ..base
    }
}
