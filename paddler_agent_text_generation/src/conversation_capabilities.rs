use crate::conversation_support::ConversationSupport;
use crate::multimodal_ingestion::MultimodalIngestion;

pub struct ConversationCapabilities {
    pub conversation_support: ConversationSupport,
    pub multimodal_ingestion: MultimodalIngestion,
}
