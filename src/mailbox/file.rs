use std::{fmt::Display, fs::File, io::{BufRead, BufReader, Error}};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::{error, info, warn};

use crate::mailbox::{Email, EmailError, EmailId, EmailReader, FileSource, MailboxError, Mbox};

pub type SeekRange = (u64, u64);

#[derive(Serialize)]
pub struct BodyFilePtr {
    content_type: String,
    content_transfer_encoding: String,
    content: SeekRange,
}
pub struct EmailFilePtr {
    email: SeekRange,
    subject: SeekRange,
    from: SeekRange,
    datetime: DateTime<Utc>,
    bodies: Vec<BodyFilePtr>,
}



impl EmailReader for EmailFilePtr {
    fn read_email(_id: &EmailId) -> Result<Email, EmailError> {
        todo!()
    }
}

impl<'a> TryFrom<FileSource<'a>> for Mbox<EmailFilePtr> {
    type Error = MailboxError;

    fn try_from(source: FileSource<'a>) -> Result<Self, Self::Error> {
        let tokens = lex(source.0)?;
        parse(&tokens).map(| emails| Mbox { emails })
    }

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

// impl Display for Token {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match *self {
//             Token::StartEmail(_) => f.write_str("StartEmail"),
//             Token::Subject(_) => f.write_str("Subject"),
//             Token::Date(_) => f.write_str("Date"),
//             Token::From(_) => f.write_str("From"),
//             Token::Boby(_) => f.write_str("Boby"),
//             Token::ContentType(_) => f.write_str("ContentType"),
//             Token::ContentTransferEncoding(_) => f.write_str("ContentTransferEncoding"),
//             Token::End(_) => f.write_str("End"),
//             Token::Continuation => f.write_str("Continuation"),
//             Token::Ignore => f.write_str("Ignore"),
//         }
//     }
// }

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
                email: self.email.unwrap(),
                subject: self.subject.unwrap(),
                from: self.from.unwrap(),
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
                        content: (*start_pos, *end_pos)
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
        let token = lex_line(seek_position, &buf[0..&buf.len()-1], &mut boundary, &current_token);
        match token {
            Token::Continuation => (),
            _ => {
                lex_push_current_token(seek_position, &mut tokens, current_token, false);
                current_token = token
            },
        };
        seek_position += read_size as u64;
        buf.clear();
    }
    lex_push_current_token(seek_position, &mut tokens, current_token, true);
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
        let tokens = lex("datasets/test_seek_positions.mbox");
        assert!(tokens.is_ok());
        let res = parse(&tokens.unwrap());
        assert!(res.is_ok());
        let emails = res.unwrap();
        println!("emails len : {}", emails.len());
        assert_eq!(1, emails.len());
        assert_eq!(25, emails[0].email.0);
    }

    #[test]
    fn test_parse_file() {
        let tokens = lex("datasets/test_lex.mbox");
        assert!(tokens.is_ok());
        let emails = parse(&tokens.unwrap());
        assert!(emails.is_ok());
        assert_eq!(3, emails.unwrap().len());
    }

    #[test]
    fn test_lex_file() {
        let tokens = lex("datasets/test_lex.mbox");
        assert!(tokens.is_ok());
        // for token in tokens.as_ref().unwrap() {
        //     println!("{token}")
        // }
        assert!(tokens.is_ok());
    }

    #[test]
    fn test_lex_line_from() {
        let mut boundary = None;
        let token = lex_line(0, "From toto@example.com\n", &mut boundary, &Token::Ignore);
        match token {
            Token::StartEmail(pos) => assert_eq!(pos, 0),
            _ => panic!("Expected StartEmail token"),
        }
    }

    #[test]
    fn test_lex_line_subject() {
        let mut boundary = None;
        let token = lex_line(10, "Subject: Hello\n", &mut boundary, &Token::Ignore);
        match token {
            Token::Subject(pos) => assert_eq!(pos, 19), // 10 + 9
            _ => panic!("Expected Subject token"),
        }
    }

    #[test]
    fn test_lex_line_date() {
        let mut boundary = None;
        let token = lex_line(5, "Date: Mon, 1 Jan 2020 00:00:00 +0000\n", &mut boundary, &Token::Ignore);
        match token {
            Token::Date(ref s) => assert_eq!(s, "Mon, 1 Jan 2020 00:00:00 +0000\n"),
            _ => panic!("Expected Date token"),
        }
    }

    #[test]
    fn test_lex_line_content_type_with_boundary() {
        let mut boundary = None;
        let token = lex_line(0, "Content-Type: multipart/mixed; boundary=\"abc123\"\n", &mut boundary, &Token::Ignore);
        assert!(matches!(token, Token::Ignore));
        assert_eq!(boundary, Some("abc123".to_string()));
    }

    #[test]
    fn test_lex_line_ignore() {
        let mut boundary = None;
        let token = lex_line(0, "Random header\n", &mut boundary, &Token::Ignore);
        assert!(matches!(token, Token::Ignore));
    }
}
