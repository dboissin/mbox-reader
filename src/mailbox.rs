use chrono::{DateTime, Utc};

use crate::{MailSearchRepository, MailStorageRepository};


pub struct Email {
    pub from: String,
    pub datetime: DateTime<Utc>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>
}

pub struct MailboxService<M:MailStorageRepository, S:MailSearchRepository> {
    storage_repository: M,
    search_repository: S,
}
