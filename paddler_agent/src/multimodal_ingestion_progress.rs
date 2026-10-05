pub enum MultimodalIngestionProgress {
    ChunksRemain { next_position: i32 },
    PromptIngested { next_position: i32 },
}
