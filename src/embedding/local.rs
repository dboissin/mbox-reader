use std::{fmt, sync::{mpsc::{self}, Arc, Mutex}, thread};

use crossbeam_channel::{bounded, Receiver, Sender};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType};
use tracing::instrument;

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

impl fmt::Debug for InternalEmbedder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InternalEmbedder")
            .field("model", &"<SentenceEmbeddingsModel>")
            .finish()
    }
}

impl Embedder for InternalEmbedder {

    #[instrument(skip_all)]
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

struct EmbeddingTask {
    idx: usize,
    contents: Vec<Arc<str>>,
}

struct EmbeddingResult {
    idx: usize,
    vectors: Vec<Vec<f32>>,
}

#[derive(Debug)]
pub struct InternalEmbedderPool {
    nb_embedders: usize,
    task_send: Sender<EmbeddingTask>,
    task_result: Receiver<EmbeddingResult>,
}

impl InternalEmbedderPool {

    pub fn new(workers: usize) -> Result<Self, EmbeddingError> {
        let (task_tx, task_rx) = bounded(100);
        let (result_tx, result_rx) = bounded(100);
        for _ in 0..workers {
            let rx:Receiver<EmbeddingTask> = task_rx.clone();
            let tx = result_tx.clone();
            thread::spawn(move || {
                let embedder = InternalEmbedder::new().unwrap();
                while let Ok(task) = rx.recv() {
                    let vectors = embedder.embed(&task.contents.iter().map(|c| c.as_ref()).collect::<Vec<&str>>()).unwrap();
                    tx.send(EmbeddingResult{ idx : task.idx, vectors }).unwrap();
                }
            });
        }
        Ok(Self { nb_embedders: workers, task_send: task_tx, task_result: result_rx })
    }

}

impl <'a> Embedder for InternalEmbedderPool {

    fn embed(&self, text: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let slice_size = (text.len() + self.nb_embedders - 1) / self.nb_embedders;
        let tasks = text.chunks(slice_size).enumerate()
            .map(|(idx, chunk)| EmbeddingTask{
                idx,
                contents: chunk.iter().map(|t| Arc::from(*t)).collect(),
            });
        let mut expected_responses = 0;
        for task in tasks {
            if self.task_send.send(task).is_err() {
                return Err(EmbeddingError::EncodeError);
            } else {
                expected_responses += 1;
            }
        }

        let mut res = Vec::with_capacity(expected_responses);
        while expected_responses > 0 {
            if let Ok(r) = self.task_result.recv() {
                res.push(r);
                expected_responses -= 1;
            } else {
                return Err(EmbeddingError::EncodeError);
            }
        }

        res.sort_by(|a, b| a.idx.cmp(&b.idx));
        let mut result = Vec::with_capacity(text.len());
        for embedding in res {
            result.extend(embedding.vectors);
        }
        Ok(result)
    }

    fn embed_line(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if let Ok(mut res) = self.embed(&[text]) {
            res.pop().ok_or(EmbeddingError::MissingResultError)
        } else {
            Err(EmbeddingError::EncodeError)
        }
    }

}

#[derive(Debug)]
pub struct InternalEmbedderModelPool {
    embedders: Vec<Arc<Mutex<InternalEmbedder>>>,
    nb_embedders: usize,
}

impl InternalEmbedderModelPool  {

    #[instrument(name = "Initialise models")]
    pub fn new(workers: usize) -> Result<Self, EmbeddingError> {
        let mut  embedders = Vec::with_capacity(workers);
        for _ in 0..workers {
            embedders.push(Arc::from(Mutex::new(InternalEmbedder::new()?)));
        }
        Ok(Self { embedders, nb_embedders: workers })
    }

}

impl Embedder for InternalEmbedderModelPool {
    fn embed(&self, text: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let slice_size = (text.len() + self.nb_embedders - 1) / self.nb_embedders;
        let tasks = text.chunks(slice_size).enumerate()
            .map(|(idx, chunk)| EmbeddingTask{
                idx,
                contents: chunk.iter().map(|t| Arc::from(*t)).collect(),
            });

        let (tx, rx) = mpsc::channel();
        let mut expected_responses = 0;
        let mut children = Vec::with_capacity(self.nb_embedders);
        for task in tasks {
            let tx_thread:mpsc::Sender<EmbeddingResult> = tx.clone();
            let embedder_mutex = self.embedders[expected_responses].clone();
            let child = thread::spawn(move || {
                let embedder = embedder_mutex.lock().unwrap();
                let vectors = embedder.embed(&task.contents.iter().map(|c| c.as_ref()).collect::<Vec<&str>>());
                tx_thread.send(EmbeddingResult { idx: task.idx, vectors: vectors.unwrap() })
            });
            children.push(child);
            expected_responses += 1;
        }

        let mut embedding_results = Vec::with_capacity(expected_responses);
        while expected_responses > 0 {
            if let Ok(r) = rx.recv() {
                embedding_results.push(r);
                expected_responses -= 1;
            } else {
                return Err(EmbeddingError::EncodeError);
            }
        }

        embedding_results.sort_by(|a, b| a.idx.cmp(&b.idx));

        let mut result = Vec::with_capacity(text.len());
        for embedding in embedding_results {
            result.extend(embedding.vectors);
        }
        for child in children {
            if child.join().is_err() {
                return Err(EmbeddingError::EncodeError);
            }
        }
        Ok(result)
    }

    fn embed_line(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
         if let Ok(mut res) = self.embed(&[text]) {
            res.pop().ok_or(EmbeddingError::MissingResultError)
        } else {
            Err(EmbeddingError::EncodeError)
        }
    }

}
