use std::error::Error;

pub mod local;

#[derive(Debug, strum::Display)]
pub enum EmbeddingError {
    ModelNotFound,
    Error,
    EncodeError,
    MissingResultError,
}

impl Error for EmbeddingError {}

pub trait Embedder {

    fn embed(&self, text: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    fn embed_line(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

}
