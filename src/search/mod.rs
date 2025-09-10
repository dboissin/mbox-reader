use std::{error::Error, fmt::Debug};

pub mod memory_cosinus;

#[derive(Debug, strum::Display)]
pub enum SearchError {
    ModelNotFound,
    Error
}

impl Error for SearchError {}

#[derive(Debug)]
pub struct SearchResult<T: PartialOrd> {
    pub id: T,
    pub score: f32,
}

pub trait MailSearchRepository: Debug {
    type EmailId: PartialOrd;

    fn index(&mut self, id: Self::EmailId, email_vector: Vec<f32>) -> Result<(), SearchError>;

    fn search(&self, ask: &[f32], nb_results: usize) -> Result<Vec<SearchResult<Self::EmailId>>, SearchError>;

}

impl<T: PartialOrd> Eq for SearchResult<T> {}

impl<T: PartialOrd> PartialEq for SearchResult<T> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl <T: PartialOrd> PartialOrd for SearchResult<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.score.partial_cmp(&self.score)
    }
}

impl <T: PartialOrd> Ord for SearchResult<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}
