use crate::Email;

pub mod file;

pub struct FileSource<'a>(pub &'a str);

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
