use std::{error::Error, fmt::{Debug, Display}};

use crate::Email;

pub mod file;

// pub struct FileSource<'a>(pub &'a str);

#[derive(Debug, PartialEq, strum::Display)]
pub enum MailboxError {
    MboxFileNotFound,
    MboxParseError,
    MboxValidationError,
    EmailNotFound,
    DecodeQuotedPrintableError,
    UTF8EncodeError,
    EncodedWordDecodeError,
}

impl Error for MailboxError {}

pub trait MailStorageRepository: Debug {
    type EmailId: PartialOrd + Display;

    fn get_email(&self, id: &Self::EmailId) -> Result<Email<Self::EmailId>, MailboxError>;

    fn count_emails(&self) -> Result<usize, MailboxError>;

    fn emails(&self) -> impl Iterator<Item = Email<Self::EmailId>>;

}
