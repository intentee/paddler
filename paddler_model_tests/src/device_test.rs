#[macro_export]
macro_rules! device_test {
    (
        $name:ident,
        |$device_binding:ident| $body:block
    ) => {
        $crate::__device_test_for!($name, cpu, Cpu, |$device_binding| $body);
        $crate::__device_test_for!($name, cuda, Cuda, |$device_binding| $body, "cuda");
        $crate::__device_test_for!($name, metal, Metal, |$device_binding| $body, "metal");
    };
}

#[macro_export]
macro_rules! gpu_device_test {
    (
        $name:ident,
        |$device_binding:ident| $body:block
    ) => {
        $crate::__device_test_for!($name, cuda, Cuda, |$device_binding| $body, "cuda");
        $crate::__device_test_for!($name, metal, Metal, |$device_binding| $body, "metal");
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __device_test_for {
    (
        $name:ident,
        $prefix:ident,
        $variant:ident,
        |$device_binding:ident| $body:block
    ) => {
        $crate::pastey::paste! {
            #[actix_web::test]
            async fn [<$prefix _ $name>]() -> ::anyhow::Result<()> {
                let $device_binding = $crate::test_device::TestDevice::$variant;
                $device_binding.require_available()?;
                $body
            }
        }
    };
    (
        $name:ident,
        $prefix:ident,
        $variant:ident,
        |$device_binding:ident| $body:block,
        $feature:literal
    ) => {
        $crate::pastey::paste! {
            #[cfg(feature = $feature)]
            #[actix_web::test]
            async fn [<$prefix _ $name>]() -> ::anyhow::Result<()> {
                let $device_binding = $crate::test_device::TestDevice::$variant;
                $device_binding.require_available()?;
                $body
            }
        }
    };
}
