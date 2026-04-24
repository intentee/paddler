use crate::test_device::TestDevice;

#[cfg(feature = "cuda")]
pub const CURRENT_TEST_DEVICE: TestDevice = TestDevice::Cuda;

#[cfg(all(not(feature = "cuda"), feature = "metal"))]
pub const CURRENT_TEST_DEVICE: TestDevice = TestDevice::Metal;

#[cfg(all(not(feature = "cuda"), not(feature = "metal")))]
pub const CURRENT_TEST_DEVICE: TestDevice = TestDevice::Cpu;
