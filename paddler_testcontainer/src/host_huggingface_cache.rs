use hf_hub::Cache;

#[must_use]
pub fn host_huggingface_cache() -> String {
    Cache::from_env().path().to_string_lossy().into_owned()
}
