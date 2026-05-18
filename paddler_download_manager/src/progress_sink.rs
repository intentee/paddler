pub trait ProgressSink: Send + Sync {
    fn on_started(&self, total_bytes: u64, already_downloaded: u64);
    fn on_chunk(&self, additional_bytes: u64);
    fn on_finished(&self);
}
