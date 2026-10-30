use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;

use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::embedding_document_duplicates::EmbeddingDocumentDuplicates;

struct DocumentsSharingContent {
    duplicate_document_ids: Vec<String>,
    first_document_id: String,
    first_position: usize,
}

struct DistinctContent {
    content: String,
    documents_sharing_content: DocumentsSharingContent,
}

fn identifies_documents_unambiguously(input_batch: &[EmbeddingInputDocument]) -> bool {
    let mut document_ids = HashSet::with_capacity(input_batch.len());

    input_batch
        .iter()
        .all(|document| document_ids.insert(document.id.as_str()))
}

pub struct DistinctEmbeddingDocuments {
    pub duplicates: EmbeddingDocumentDuplicates,
    pub params: GenerateEmbeddingBatchParams,
}

impl From<GenerateEmbeddingBatchParams> for DistinctEmbeddingDocuments {
    fn from(params: GenerateEmbeddingBatchParams) -> Self {
        if !identifies_documents_unambiguously(&params.input_batch) {
            return Self {
                duplicates: EmbeddingDocumentDuplicates::default(),
                params,
            };
        }

        let GenerateEmbeddingBatchParams {
            input_batch,
            normalization_method,
        } = params;
        let mut documents_by_content: HashMap<String, DocumentsSharingContent> =
            HashMap::with_capacity(input_batch.len());

        for (position, EmbeddingInputDocument { content, id }) in
            input_batch.into_iter().enumerate()
        {
            match documents_by_content.entry(content) {
                Entry::Occupied(mut documents_sharing_content) => {
                    documents_sharing_content
                        .get_mut()
                        .duplicate_document_ids
                        .push(id);
                }
                Entry::Vacant(first_with_content) => {
                    first_with_content.insert(DocumentsSharingContent {
                        duplicate_document_ids: Vec::new(),
                        first_document_id: id,
                        first_position: position,
                    });
                }
            }
        }

        let mut distinct_contents: Vec<DistinctContent> = documents_by_content
            .into_iter()
            .map(|(content, documents_sharing_content)| DistinctContent {
                content,
                documents_sharing_content,
            })
            .collect();

        distinct_contents.sort_by_key(|distinct_content| {
            distinct_content.documents_sharing_content.first_position
        });

        let mut duplicate_document_ids = HashMap::new();
        let mut distinct_documents = Vec::with_capacity(distinct_contents.len());

        for DistinctContent {
            content,
            documents_sharing_content:
                DocumentsSharingContent {
                    duplicate_document_ids: duplicates_of_first_document,
                    first_document_id,
                    ..
                },
        } in distinct_contents
        {
            if !duplicates_of_first_document.is_empty() {
                duplicate_document_ids
                    .insert(first_document_id.clone(), duplicates_of_first_document);
            }

            distinct_documents.push(EmbeddingInputDocument {
                content,
                id: first_document_id,
            });
        }

        Self {
            duplicates: EmbeddingDocumentDuplicates {
                duplicate_document_ids,
            },
            params: GenerateEmbeddingBatchParams {
                input_batch: distinct_documents,
                normalization_method,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

    use super::DistinctEmbeddingDocuments;
    use crate::embedding_document_duplicates::EmbeddingDocumentDuplicates;

    fn document(id: &str, content: &str) -> EmbeddingInputDocument {
        EmbeddingInputDocument {
            content: content.to_owned(),
            id: id.to_owned(),
        }
    }

    fn batch_of(input_batch: Vec<EmbeddingInputDocument>) -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch,
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[test]
    fn embeds_each_content_once_for_every_document_sharing_it() {
        let DistinctEmbeddingDocuments { duplicates, params } =
            DistinctEmbeddingDocuments::from(batch_of(vec![
                document("first", "shared content"),
                document("lone", "lone content"),
                document("second", "shared content"),
                document("third", "shared content"),
            ]));

        assert_eq!(
            params.input_batch,
            vec![
                document("first", "shared content"),
                document("lone", "lone content"),
            ]
        );
        assert_eq!(
            duplicates,
            EmbeddingDocumentDuplicates {
                duplicate_document_ids: HashMap::from([(
                    "first".to_owned(),
                    vec!["second".to_owned(), "third".to_owned()],
                )]),
            }
        );
    }

    #[test]
    fn embeds_every_document_of_a_batch_that_reuses_a_document_id() {
        let ambiguous_batch = vec![
            document("reused", "shared content"),
            document("reused", "other content"),
            document("distinct", "shared content"),
        ];
        let DistinctEmbeddingDocuments { duplicates, params } =
            DistinctEmbeddingDocuments::from(batch_of(ambiguous_batch.clone()));

        assert_eq!(params.input_batch, ambiguous_batch);
        assert_eq!(duplicates, EmbeddingDocumentDuplicates::default());
    }
}
