use std::{collections::{BinaryHeap, HashMap}, fmt::Debug, hash::Hash};

use tracing::{debug, instrument};

use crate::{MailSearchRepository, SearchResult};

#[derive(Debug)]
pub struct MemoryCosinus<T:Debug> {
    vectors: HashMap<T, Vec<f32>>
}

impl <T:Debug> MemoryCosinus<T> {

    pub fn new() -> MemoryCosinus<T> {
        Self { vectors: HashMap::new() }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

}

impl <T: Hash + Eq + PartialOrd + Clone + Debug> MailSearchRepository for MemoryCosinus<T> {
    type EmailId = T;

    fn index(&mut self, id: Self::EmailId, email_vector: Vec<f32>) -> Result<(), super::SearchError> {
        self.vectors.insert(id, email_vector);
        Ok(())
    }

    #[instrument(skip_all, fields(nb_resultats = %nb_results))]
    fn search(&self, ask: &[f32], nb_results: usize) -> Result<Vec<SearchResult<Self::EmailId>>, super::SearchError> {
        let mut scores = BinaryHeap::with_capacity(nb_results);
        for (id, vector) in &self.vectors {
            let score = Self::cosine_similarity(ask, vector);
            if scores.len() < nb_results {
                scores.push(SearchResult{id: id.clone(), score});
            } else if let Some(min_result) = scores.peek() && min_result.score < score {
                debug!("Replace score {} by {}", &min_result.score, &score);
                scores.pop();
                scores.push(SearchResult{id: id.clone(), score});
            }
        }
        let mut res: Vec<SearchResult<_>> = scores.into_iter().collect();
        res.sort();
        Ok(res)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_basic() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = MemoryCosinus::<usize>::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);

        let a = vec![1.0, 1.0];
        let b = vec![1.0, 1.0];
        let sim = MemoryCosinus::<usize>::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_index_and_search() {
        let mut repo = MemoryCosinus::<usize>::new();
        repo.index(1, vec![1.0, 0.0]).unwrap();
        repo.index(2, vec![0.0, 1.0]).unwrap();
        repo.index(3, vec![1.0, 1.0]).unwrap();

        let results = repo.search(&[1.0, 0.0], 2).unwrap();
        let ids: Vec<usize> = results.iter().map(|r| r.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_with_zero_vector() {
        let mut repo = MemoryCosinus::<usize>::new();
        repo.index(1, vec![0.0, 0.0]).unwrap();
        let results = repo.search(&[1.0, 0.0], 1).unwrap();
        assert_eq!(results[0].score, 0.0);
    }
}