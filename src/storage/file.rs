use std::{fs::{File}, io::{BufRead, BufReader, Error}, ops::Range};

use chrono::{DateTime, Utc};
use memmap2::Mmap;
use quoted_printable::{decode, ParseMode};
use rfc2047_decoder::{Decoder, RecoverStrategy};
use serde::Serialize;
use tracing::{error, warn};

use crate::{storage::MailboxError, Email, MailStorageRepository};


pub type SeekRange = (u64, u64);

pub struct MboxFile {
    emails: Vec<EmailFilePtr>,
    file_mmap: Mmap,
}

#[derive(Serialize)]
struct BodyFilePtr {
    content_type: String,
    content_transfer_encoding: String,
    content: Range<usize>,
}

impl BodyFilePtr {

    fn is_html(&self) -> bool {
        self.content_type.contains("text/html")
    }

}

struct EmailFilePtr {
    email: Range<usize>,
    subject: Range<usize>,
    from: Range<usize>,
    datetime: DateTime<Utc>,
    bodies: Vec<BodyFilePtr>,
}

impl From<Error> for MailboxError {
    fn from(_e: Error) -> Self {
        // TODO add match for different type error
        MailboxError::MboxFileNotFound
    }
}

enum Token {
    StartEmail(u64),
    Subject(u64),
    Date(String),
    From(u64),
    Boby(u64),
    ContentType(String),
    ContentTransferEncoding(String),
    End(u64),
    Continuation,
    Ignore
}

#[derive(Serialize)]
struct EmailFilePtrValidator {
    email: Option<SeekRange>,
    subject: Option<SeekRange>,
    from: Option<SeekRange>,
    datetime: Option<DateTime<Utc>>,
    bodies: Vec<BodyFilePtr>,
}

impl EmailFilePtrValidator {
    fn new() -> Self {
        Self { email: None, subject: None, from: None, datetime: None, bodies: vec![] }
    }
    
    fn validate(self) -> Result<EmailFilePtr, MailboxError> {
        if self.email.is_some() && self.subject.is_some() && self.from.is_some() && self.datetime.is_some() {
            Ok(EmailFilePtr{
                email: Range { start: self.email.unwrap().0 as usize, end: self.email.unwrap().1 as usize },
                subject: Range { start: self.subject.unwrap().0 as usize, end: self.subject.unwrap().1 as usize },
                from: Range { start: self.from.unwrap().0 as usize, end: self.from.unwrap().1 as usize },
                datetime: self.datetime.unwrap(),
                bodies: self.bodies
            })
        } else {
            if let Ok(value) = serde_json::to_string(&self) {
                warn!("EmailFilePtr validation failed  : {value}");
            } else {
                warn!("EmailFilePtr validation failed.");
            }
            Err(MailboxError::MboxValidationError)
        }
    }
}

impl MboxFile {

    pub fn new(file_path: &str) -> Result<Self, MailboxError> {
        let tokens = Self::lex(file_path)?;
        let file_mmap = unsafe {
            // unsafe block require in case of file is truncated while in use
            Mmap::map(&File::open(file_path)?)?
        };
        Self::parse(&tokens).map(| emails| MboxFile { emails, file_mmap })
    }

    fn parse(tokens: &[Token]) -> Result<Vec<EmailFilePtr>, MailboxError> {
        let mut emails = vec![];
        let mut stack: Vec<&Token> = vec![];
        let mut validator = EmailFilePtrValidator::new();

        for token in tokens {
            match token {
                Token::StartEmail(_) if !stack.is_empty() => {
                    error!("Invalid email missing tokens.");
                    stack.clear();
                },
                Token::End(end_pos) => match stack.pop() {
                    Some(Token::Boby(start_pos)) => {
                        let (cte, ct) = if stack.len() > 2 {
                            match (stack.pop().unwrap(), stack.pop().unwrap()) {
                                (Token::ContentTransferEncoding(cte), Token::ContentType(ct)) =>
                                    (cte.as_str(), ct.as_str()),
                                (a,b) => {
                                    stack.push(b);
                                    stack.push(a);
                                    ("quoted-printable", "text/plain")
                                }
                            }
                        } else {
                            ("quoted-printable", "text/plain")
                        };
                        validator.bodies.push(BodyFilePtr {
                            content_type: ct.to_string(),
                            content_transfer_encoding: cte.to_string(),
                            content: Range{ start: *start_pos as usize, end : *end_pos as usize}
                        })
                    },
                    Some(Token::From(start_pos)) =>
                        validator.from = Some((*start_pos, *end_pos)),
                    Some(Token::Subject(start_pos)) =>
                        validator.subject = Some((*start_pos, *end_pos)),
                    Some(Token::StartEmail(start_pos)) => {
                        validator.email = Some((*start_pos, *end_pos));
                        let tmp = validator;
                        validator = EmailFilePtrValidator::new();
                        if let Ok(email_ptr) = tmp.validate() {
                            emails.push(email_ptr);
                        }
                    },
                    _ => return  Err(MailboxError::MboxParseError),
                },
                Token::Date(date) => {
                    validator.datetime = DateTime::parse_from_rfc2822(&date).ok()
                            .map(|dt| dt.to_utc());
                },
                Token::ContentTransferEncoding(_) | Token::ContentType(_) => (),
                _ => stack.push(token),

            };
        }
        Ok(emails)
    }

    fn lex(file_path: &str) -> Result<Vec<Token>, MailboxError> {
        let mut file_reader = BufReader::new(File::open(file_path)?);
        let mut seek_position:u64 = 0;
        let mut buf = String::new();
        let mut tokens = vec![];
        let mut boundary = None;
        let mut current_token = Token::Ignore;

        while let Ok(read_size) = file_reader.read_line(&mut buf) && read_size > 0 {
            let token = Self::lex_line(seek_position, &buf[0..&buf.len()-1], &mut boundary, &current_token);
            match token {
                Token::Continuation => (),
                _ => {
                    Self::lex_push_current_token(seek_position, &mut tokens, current_token, false);
                    current_token = token
                },
            };
            seek_position += read_size as u64;
            buf.clear();
        }
        Self::lex_push_current_token(seek_position, &mut tokens, current_token, true);
        Ok(tokens)
    }

    fn lex_push_current_token(seek_position: u64, tokens: &mut Vec<Token>, current_token: Token, end: bool) {
        match current_token {
            Token::StartEmail(_) if end => (),
            Token::StartEmail(position) if position > 0 => {
                tokens.push(Token::End(position));
                tokens.push(current_token);
            }
            Token::Ignore | Token::Continuation => (),
            Token::End(_) | Token::StartEmail(_) | Token::ContentType(_) |
                Token::Date(_) | Token::ContentTransferEncoding(_) => tokens.push(current_token),
            Token::From(_) | Token::Subject(_) | Token::Boby(_) => {
                tokens.push(current_token);
                tokens.push(Token::End(seek_position));
            }
        }
        if end {
            tokens.push(Token::End(seek_position));
        }
    }

    fn lex_line(seek_position: u64, buf: &str, boundary: &mut Option<String>, current_token: &Token) -> Token {
        if buf.starts_with("From ") {
            *boundary = None;
            Token::StartEmail(seek_position)
        } else if buf.starts_with("Subject: ") {
            Token::Subject(seek_position + 9)
        } else if buf.starts_with("From: ") {
            Token::From(seek_position + 6)
        } else if buf.starts_with("Date: ") {
            Token::Date(buf[6..].to_string())
        } else if buf.starts_with("Content-Transfer-Encoding: ") {
            Token::ContentTransferEncoding(buf[27..].to_string())
        } else if buf.starts_with("Content-Type: ") {
            if let Some(nb) = buf.find("boundary=") {
                let tmp_boundary = &buf[(nb+10)..];
                if let Some(idx) = tmp_boundary.find('"') {
                    *boundary = Some((&tmp_boundary[0..idx]).to_string())
                }
                Token::Ignore
            } else {
                Token::ContentType(buf[14..].to_string())
            }
        } else if let Some(boundary) = &boundary && buf.starts_with(boundary) {
            Token::End(seek_position)
        } else {
            match *current_token {
                Token::ContentTransferEncoding(_) if buf.is_empty() => Token::Boby(seek_position),
                Token::Boby(_) => Token::Continuation,
                _ if boundary.is_none() && buf.is_empty() => Token::Boby(seek_position),
                _ if buf.starts_with(" ") => Token::Continuation,
                _ => Token::Ignore
            }
        }
    }

    fn get_header(&self, range: &Range<usize>) -> Result<String, MailboxError> {
        let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Skip);
        decoder.decode(&self.file_mmap[range.start..range.end])
            .map(|value| value.replace("\n", ""))
            .or(Err(MailboxError::EncodedWordDecodeError))
    }

    fn get_body(&self, body_ptr: &BodyFilePtr) -> Result<String, MailboxError> {
        if let Ok(decoded) = decode(&self.file_mmap[body_ptr.content.start..body_ptr.content.end], ParseMode::Robust) {
            String::from_utf8(decoded).or(Err(MailboxError::UTF8EncodeError))
        } else {
            Err(MailboxError::DecodeQuotedPrintableError)
        }
    }

}

impl MailStorageRepository for MboxFile {
    type EmailId = usize;

    fn get_email(&self, id: &Self::EmailId) -> Result<Email<Self::EmailId>, MailboxError> {
        if let Some(email_ptr) = self.emails.get(*id) {
            let email = Email {
                id: *id,
                from: self.get_header(&email_ptr.from)?,
                datetime: email_ptr.datetime,
                subject: self.get_header(&email_ptr.subject)?,
                body_text: email_ptr.bodies.iter()
                            .filter(|bp| !bp.is_html())
                            .next()
                            .map(|bp| self.get_body(bp).ok())
                            .flatten(),
                body_html: email_ptr.bodies.iter()
                            .filter(|bp| bp.is_html())
                            .next()
                            .map(|bp| self.get_body(bp).ok())
                            .flatten()
            };
            Ok(email)
        } else {
            Err(MailboxError::EmailNotFound)
        }
    }

    fn count_emails(&self) -> Result<usize, MailboxError> {
        Ok(self.emails.len())
    }

    fn emails(&self) -> impl Iterator<Item = Email<Self::EmailId>> {
        EmailIterator { idx: 0, mbox: &self }
    }

}

struct EmailIterator<'a> {
    idx: usize,
    mbox: &'a MboxFile,
}

impl<'a> Iterator for EmailIterator<'a> {
    type Item = Email<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.mbox.get_email(&self.idx).ok();
        self.idx += 1;
        res
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;

    #[test]
    fn test_datetime() {
        let date = "Mon, 4 Aug 2025 11:56:07 +0800";
        let datetime = DateTime::parse_from_rfc2822(&date).ok()
                        .map(|dt| dt.to_utc());
        assert!(datetime.is_some());
    }

    #[test]
    #[traced_test]
    fn test_seek_positions() {
        let tokens = MboxFile::lex("datasets/test_seek_positions.mbox");
        assert!(tokens.is_ok());
        let res = MboxFile::parse(&tokens.unwrap());
        assert!(res.is_ok());
        let emails = res.unwrap();
        println!("emails len : {}", emails.len());
        assert_eq!(1, emails.len());
        assert_eq!(25, emails[0].email.start);
    }

    #[test]
    fn test_parse_file() {
        let tokens = MboxFile::lex("datasets/test_lex.mbox");
        assert!(tokens.is_ok());
        let emails = MboxFile::parse(&tokens.unwrap());
        assert!(emails.is_ok());
        assert_eq!(3, emails.unwrap().len());
    }

    #[test]
    fn test_lex_file() {
        let tokens = MboxFile::lex("datasets/test_lex.mbox");
        assert!(tokens.is_ok());
        // for token in tokens.as_ref().unwrap() {
        //     println!("{token}")
        // }
        assert!(tokens.is_ok());
    }

    #[test]
    fn test_lex_line_from() {
        let mut boundary = None;
        let token = MboxFile::lex_line(0, "From toto@example.com\n", &mut boundary, &Token::Ignore);
        match token {
            Token::StartEmail(pos) => assert_eq!(pos, 0),
            _ => panic!("Expected StartEmail token"),
        }
    }

    #[test]
    fn test_lex_line_subject() {
        let mut boundary = None;
        let token = MboxFile::lex_line(10, "Subject: Hello\n", &mut boundary, &Token::Ignore);
        match token {
            Token::Subject(pos) => assert_eq!(pos, 19), // 10 + 9
            _ => panic!("Expected Subject token"),
        }
    }

    #[test]
    fn test_lex_line_date() {
        let mut boundary = None;
        let token = MboxFile::lex_line(5, "Date: Mon, 1 Jan 2020 00:00:00 +0000\n", &mut boundary, &Token::Ignore);
        match token {
            Token::Date(ref s) => assert_eq!(s, "Mon, 1 Jan 2020 00:00:00 +0000\n"),
            _ => panic!("Expected Date token"),
        }
    }

    #[test]
    fn test_lex_line_content_type_with_boundary() {
        let mut boundary = None;
        let token = MboxFile::lex_line(0, "Content-Type: multipart/mixed; boundary=\"abc123\"\n", &mut boundary, &Token::Ignore);
        assert!(matches!(token, Token::Ignore));
        assert_eq!(boundary, Some("abc123".to_string()));
    }

    #[test]
    fn test_lex_line_ignore() {
        let mut boundary = None;
        let token = MboxFile::lex_line(0, "Random header\n", &mut boundary, &Token::Ignore);
        assert!(matches!(token, Token::Ignore));
    }
}
