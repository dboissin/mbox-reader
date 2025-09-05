pub mod local;

#[derive(Debug)]
pub enum EmbeddingError {
    ModelNotFound,
    Error,
    EncodeError,
    MissingResultError,
}

pub trait Embedder {

    fn embed(&self, text: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    fn embed_line(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

}
