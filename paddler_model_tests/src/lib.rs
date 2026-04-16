pub mod collect_generated_tokens;
pub mod device_test;
pub mod load_test_image_as_data_uri;
pub mod log_generated_response;
pub mod managed_model;
pub mod managed_model_params;
pub mod model_test_harness;
pub mod test_device;
#[cfg(feature = "cuda")]
pub mod test_device_cuda;
#[cfg(feature = "metal")]
pub mod test_device_metal;

#[doc(hidden)]
pub use paste;
