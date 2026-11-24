use paddler_messaging::embedding_input_document::EmbeddingInputDocument;

#[must_use]
pub fn numbered_embedding_documents(document_count: usize) -> Vec<EmbeddingInputDocument> {
    (0..document_count)
        .map(|document_index| EmbeddingInputDocument {
            content: format!("Document number {document_index}."),
            id: format!("doc-{document_index}"),
        })
        .collect()
}
