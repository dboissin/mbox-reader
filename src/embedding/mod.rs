pub mod local;

pub enum EmbeddingError {
    ModelNotFound,
    Error
}

pub trait Embedder {

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

}
