pub struct IngestingContribution {
    pub request_index: usize,
    pub chunk_size: usize,
    pub is_last_chunk: bool,
    pub last_batch_position: i32,
}
