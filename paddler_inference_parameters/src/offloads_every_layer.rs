use crate::constants::ALL_GPU_LAYERS;

#[must_use]
pub const fn offloads_every_layer(n_gpu_layers: i32) -> bool {
    n_gpu_layers == ALL_GPU_LAYERS
}
