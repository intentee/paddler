use crate::inference_capabilities::InferenceCapabilities;
use crate::served_decisions::ServedDecisions;
use crate::served_embeddings::ServedEmbeddings;
use crate::served_text_generation::ServedTextGeneration;

pub type ServedCapabilities =
    InferenceCapabilities<ServedDecisions, ServedEmbeddings, ServedTextGeneration>;
