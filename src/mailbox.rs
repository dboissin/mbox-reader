use chrono::{DateTime, Utc};

pub mod file;

pub struct FileSource<'a>(pub &'a str);

pub struct Email {
    pub from: String,
    pub datetime: DateTime<Utc>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>
}

pub enum EmailError {
    NotFound,
    ReadError
}

#[derive(Debug, PartialEq)]
pub enum MailboxError {
    MboxFileNotFound,
    MboxParseError,
    MboxValidationError,
    EmailNotFound,
    DecodeQuotedPrintableError,
    UTF8EncodeError,
    EncodedWordDecodeError,
}

pub trait MailStorageRepository {
    type EmailId;

    fn get_email(&self, id: &Self::EmailId) -> Result<Email, MailboxError>;

    fn count_emails(&self) -> Result<usize, MailboxError>;

}

pub trait MailSearchRepository {
    type EmailId;


}

pub struct MailboxService<M:MailStorageRepository> {
    storage: M,
}
