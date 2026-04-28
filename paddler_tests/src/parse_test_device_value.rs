use anyhow::Result;
use anyhow::anyhow;

use crate::test_device::TestDevice;

pub fn parse_test_device_value(value: Option<&str>) -> Result<TestDevice> {
    match value {
        None | Some("cpu") => Ok(TestDevice::Cpu),
        #[cfg(feature = "cuda")]
        Some("cuda") => Ok(TestDevice::Cuda),
        #[cfg(not(feature = "cuda"))]
        Some("cuda") => Err(anyhow!(
            "PADDLER_TEST_DEVICE=cuda requires building with --features cuda; the cuda backend is not linked into this binary"
        )),
        #[cfg(feature = "metal")]
        Some("metal") => Ok(TestDevice::Metal),
        #[cfg(not(feature = "metal"))]
        Some("metal") => Err(anyhow!(
            "PADDLER_TEST_DEVICE=metal requires building with --features metal; the metal backend is not linked into this binary"
        )),
        Some(other) => Err(anyhow!(
            "unrecognised PADDLER_TEST_DEVICE value {other:?}; expected one of cpu | cuda | metal"
        )),
    }
}
