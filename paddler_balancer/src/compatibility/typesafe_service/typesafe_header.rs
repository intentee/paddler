pub struct TypeSafeHeader;

impl TypeSafeHeader {
    pub const REQUEST_ID: &str = "x-typesafe-request-id";
    pub const RETRY_COUNT: &str = "x-typesafe-retry-count";
    pub const RUNTIME: &str = "x-typesafe-runtime";
    pub const SDK: &str = "x-typesafe-sdk";
}
