use std::{error::Error, fmt::Display};

use chrono::{DateTime, Utc};
use tracing::error;

use crate::{embedding::{local::InternalEmbedder, Embedder}, search::memory_cosinus::MemoryCosinus, storage::{file::MboxFile}, MailSearchRepository, MailStorageRepository};

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

pub struct MailboxService<T:MailStorageRepository> {
    storage_repository: T,
    search_repository: Box<dyn MailSearchRepository<EmailId = <T as MailStorageRepository>::EmailId>>,
    embedder: Box<dyn Embedder>,
}

impl <T:MailStorageRepository> MailboxService<T> {

    pub fn index_emails(&mut self) {
        for email in self.storage_repository.emails() {
            if let Ok(vector) = self.embedder.embed_line(&email.body_text.unwrap()) {
                if self.search_repository.index(email.id, vector).is_err() {
                    error!("Error when store search embedding of email");
                }
            } else {
                error!("Error when calculate embeddind of email : {}", email.id);
            }
        }
    }

    const LIMIT_SEARCH_RESULTS: usize = 5;

    pub fn search_email(&self, search_request: &str) -> Result<Vec<(f32, Email<<T as MailStorageRepository>::EmailId>)>> {
        let embedded_request = self.embedder.embed_line(search_request)?;
        let emails_idx = self.search_repository.search(&embedded_request, Self::LIMIT_SEARCH_RESULTS)?;
        let mut res = Vec::with_capacity(Self::LIMIT_SEARCH_RESULTS);
        for email_idx in emails_idx {
            res.push((email_idx.score, self.storage_repository.get_email(&email_idx.id)?));
        }
        Ok(res)
    }

}

impl<'a> TryFrom<&str> for MailboxService<MboxFile> {
    type Error = MailboxServiceError;

    fn try_from(source: &str) -> std::result::Result<Self, Self::Error> {
        if let Ok(embedder) = InternalEmbedder::new() {
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
