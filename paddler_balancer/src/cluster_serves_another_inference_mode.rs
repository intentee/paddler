use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use paddler_messaging::inference_mode::InferenceMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterServesAnotherInferenceMode {
    pub requested_inference_mode: InferenceMode,
    pub served_inference_mode: InferenceMode,
}

impl Display for ClusterServesAnotherInferenceMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let Self {
            requested_inference_mode,
            served_inference_mode,
        } = self;

        write!(
            formatter,
            "The cluster serves {served_inference_mode:?}, not {requested_inference_mode:?}"
        )
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::inference_mode::InferenceMode;

    use super::ClusterServesAnotherInferenceMode;

    #[test]
    fn names_the_served_and_the_requested_inference_mode() {
        assert_eq!(
            ClusterServesAnotherInferenceMode {
                requested_inference_mode: InferenceMode::TextGeneration,
                served_inference_mode: InferenceMode::Embeddings,
            }
            .to_string(),
            "The cluster serves Embeddings, not TextGeneration"
        );
    }
}
