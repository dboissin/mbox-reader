use std::{error::Error, fmt::Display};

use chrono::{DateTime, Utc};
use tracing::{error, instrument};

use crate::{embedding::{local::{InternalEmbedder, InternalEmbedderModelPool, InternalEmbedderPool}, Embedder}, search::memory_cosinus::MemoryCosinus, storage::file::MboxFile, MailSearchRepository, MailStorageRepository};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, strum::Display)]
pub enum MailboxServiceError {
    InitError,
    SearchError,
}

impl Error for MailboxServiceError { }


pub struct Email<EmailId> {
    pub id: EmailId,
    pub from: String,
    pub datetime: DateTime<Utc>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>
}

impl<EmailId: Display> Display for Email<EmailId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}, {}, {}, {} ", &self.id, &self.from, &self.datetime, &self.subject,
            self.body_text.as_ref().unwrap_or(&"none".to_string()),
            self.body_html.as_ref().unwrap_or(&"none".to_string())
        )
    }
}

#[derive(Debug)]
pub struct MailboxService<T:MailStorageRepository> {
    storage_repository: T,
    search_repository: Box<dyn MailSearchRepository<EmailId = <T as MailStorageRepository>::EmailId>>,
    embedder: Box<dyn Embedder>,
}

impl <T:MailStorageRepository> MailboxService<T> {

    #[instrument(skip_all)]
    pub fn index_emails(&mut self) {
        const INDEX_BUFFER_SIZE: usize = 600;

        let mut emails_iterator = self.storage_repository.emails();
        loop {
            let mut buf: Vec<Email<<T as MailStorageRepository>::EmailId>> = Vec::with_capacity(INDEX_BUFFER_SIZE);
            for _ in 0..INDEX_BUFFER_SIZE {
                if let Some(email) = emails_iterator.next() {
                    buf.push(email);
                } else {
                    break;
                }
            }
            if buf.is_empty() {
                break;
            }

            let (mut ids, bodies) = self.emails_to_ids_and_bodies_if_body_exists(buf);
            let bodies_str:Vec<&str> = bodies.iter().map(|body| body.as_str()).collect();

            if let Ok(mut vectors) = self.embedder.embed(&bodies_str) && vectors.len() == ids.len() {
                while let Some(id) = ids.pop() && let Some(vector) = vectors.pop() {
                    if self.search_repository.index(id, vector).is_err() {
                        error!("Error when store search embedding of email");
                    }
                }
            } else {
                error!("Error when calculate embeddind of emails : {}", ids.iter().map(|id| id.to_string()).collect::<String>());
            }
        }
    }

    #[instrument(skip_all, fields(user_search_input=%search_request))]
    pub fn search_email(&self, search_request: &str) -> Result<Vec<(f32, Email<<T as MailStorageRepository>::EmailId>)>> {
        const LIMIT_SEARCH_RESULTS: usize = 5;
        let embedded_request = self.embedder.embed_line(search_request)?;
        let emails_idx = self.search_repository.search(&embedded_request, LIMIT_SEARCH_RESULTS)?;
        let mut res = Vec::with_capacity(LIMIT_SEARCH_RESULTS);
        for email_idx in emails_idx {
            res.push((email_idx.score, self.storage_repository.get_email(&email_idx.id)?));
        }
        Ok(res)
    }

    fn emails_to_ids_and_bodies_if_body_exists(&self, buf: Vec<Email<<T as MailStorageRepository>::EmailId>>) -> (Vec<<T as MailStorageRepository>::EmailId>, Vec<String>) {
        buf.into_iter()
            .filter_map(|email|
                if let Some(body_text) = email.body_text {
                    Some((email.id, body_text))
                } else if let Some(body_html) = email.body_html  {
                    Some((email.id, body_html))
                } else {
                    None
                }
            )
            .unzip()
    }

}



impl<'a> TryFrom<&str> for MailboxService<MboxFile> {
    type Error = MailboxServiceError;

    fn try_from(source: &str) -> std::result::Result<Self, Self::Error> {
        // if let Ok(embedder) = time_it!("Init internal embedder", { InternalEmbedder::new() }) {
        if let Ok(embedder) = InternalEmbedderModelPool::new(4) {
            MboxFile::new(source)
                .map(|s| MailboxService {
                    storage_repository: s,
                    search_repository: Box::new(MemoryCosinus::new()),
                    embedder: Box::new(embedder)
                }).or(Err(MailboxServiceError::InitError))
        } else {
            Err(MailboxServiceError::InitError)
        }
    }

}
