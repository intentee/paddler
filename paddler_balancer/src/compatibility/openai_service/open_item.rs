#[derive(Default)]
pub enum OpenItem {
    Message {
        item_id: String,
        text: String,
    },
    #[default]
    Nothing,
    Reasoning {
        item_id: String,
        text: String,
    },
}
