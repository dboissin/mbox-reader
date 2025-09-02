pub mod file;

pub struct EmailId(usize);

pub struct FileSource<'a>(pub &'a str);

pub struct Email {
    _from: String,
    _to: String,
    _subject: String,
    _body_text: Option<String>,
    _body_html: Option<String>
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
}

pub trait Mailbox {
    
}

pub trait EmailReader {

    fn read_email(id: &EmailId) -> Result<Email, EmailError>;

}

pub struct Mbox<T:EmailReader> {
    emails: Vec<T>
}
