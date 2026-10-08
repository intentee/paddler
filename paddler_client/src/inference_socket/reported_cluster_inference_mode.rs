use paddler_messaging::inference_mode::InferenceMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportedClusterInferenceMode {
    NotYetReported,
    Reported(InferenceMode),
}
