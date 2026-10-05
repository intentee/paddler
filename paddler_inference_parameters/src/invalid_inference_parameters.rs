#[derive(Debug, PartialEq, thiserror::Error)]
pub enum InvalidInferenceParameters {
    #[error("n_batch must be at least 1")]
    BatchSizeZero,
    #[error("n_batch {n_batch} exceeds the largest batch llama.cpp accepts ({max_batch_size})")]
    BatchSizeTooLarge { n_batch: u32, max_batch_size: u32 },
    #[error("n_batch {n_batch} exceeds context_size {context_size}")]
    BatchSizeExceedsContextSize { n_batch: u32, context_size: u32 },
    #[error("n_gpu_layers {n_gpu_layers} is below -1; use -1 to offload all layers")]
    GpuLayersBelowAllLayers { n_gpu_layers: i32 },
    #[error("min_p {min_p} is outside the range 0 to 1")]
    MinPOutOfRange { min_p: f32 },
    #[error(
        "penalty strengths (repeat {penalty_repeat}, frequency {penalty_frequency}, presence {penalty_presence}) are set, but penalty_last_n is 0, which disables penalties"
    )]
    PenaltiesWithoutWindow {
        penalty_frequency: f32,
        penalty_presence: f32,
        penalty_repeat: f32,
    },
    #[error("penalty_frequency {penalty_frequency} is not a finite number")]
    PenaltyFrequencyNotFinite { penalty_frequency: f32 },
    #[error("penalty_last_n {penalty_last_n} is negative; use 0 to disable penalties")]
    PenaltyLastNNegative { penalty_last_n: i32 },
    #[error("penalty_presence {penalty_presence} is not a finite number")]
    PenaltyPresenceNotFinite { penalty_presence: f32 },
    #[error("penalty_repeat {penalty_repeat} must be a finite number greater than 0")]
    PenaltyRepeatNotPositive { penalty_repeat: f32 },
    #[error(
        "penalty_last_n {penalty_last_n} is set, but every penalty strength is neutral, so no penalty applies"
    )]
    PenaltyWindowWithoutPenalties { penalty_last_n: i32 },
    #[error("temperature {temperature} must be a finite number of at least 0")]
    TemperatureNegative { temperature: f32 },
    #[error("top_k {top_k} is negative; use 0 to disable top-k sampling")]
    TopKNegative { top_k: i32 },
    #[error("top_p {top_p} is outside the range 0 to 1")]
    TopPOutOfRange { top_p: f32 },
}
