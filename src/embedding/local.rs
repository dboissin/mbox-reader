use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType};

use crate::embedding::{Embedder, EmbeddingError};


pub struct InternalEmbedder {
    model: SentenceEmbeddingsModel,
}

impl InternalEmbedder {

    pub fn new() -> Result<Self, EmbeddingError> {
        if let Ok(model) = SentenceEmbeddingsBuilder::remote(
                SentenceEmbeddingsModelType::AllMiniLmL12V2).create_model() {
            Ok(Self { model })
        } else {
            Err(EmbeddingError::ModelNotFound)
        }
    }

}

impl Embedder for InternalEmbedder {

    fn embed(&self, text: &[&str]) -> Result<Vec<Vec<f32>>, super::EmbeddingError> {
        self.model.encode(text).or(Err(EmbeddingError::EncodeError))
    }

    fn embed_line(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if let Ok(mut res) = self.embed(&[text]) {
            res.pop().ok_or(EmbeddingError::MissingResultError)
        } else {
            Err(EmbeddingError::EncodeError)
        }
    }

}
