#[derive(Debug, Eq, PartialEq)]
pub enum CacheEntryState {
    Cached,
    Vacant,
}
