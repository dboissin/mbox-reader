
use mbox_viewer::mailbox::{file::MboxFile, FileSource, MailStorageRepository, MailboxError};
use tracing_test::traced_test;

#[test]
fn test_not_exists_mbox_file() {
    let mbox:Result<MboxFile,MailboxError> = FileSource("/chemin/vers/fichier/inexistant").try_into();
    assert!(mbox.is_err());
    assert!(mbox.as_ref().is_err_and(|e| *e == MailboxError::MboxFileNotFound));
    assert!(mbox.as_ref().is_err_and(|e| *e != MailboxError::MboxParseError));
}

#[test]
fn test_exists_mbox_file() {
    let mbox:Result<MboxFile,MailboxError> = FileSource("datasets/dev_apisix_apache_org.mbox").try_into();
    assert!(mbox.is_ok());
}

#[test]
fn test_count_emails() {
    let email_repository = MboxFile::new("datasets/test_lex.mbox").unwrap();
    assert_eq!(3, email_repository.count_emails().unwrap())
}

#[test]
fn test_get_email() {
    let email_repository = MboxFile::new("datasets/test_lex.mbox").unwrap();
    let email = email_repository.get_email(&1).unwrap();
    assert_eq!("Re: [VOTE] Apache apisix-ingress-controller release version 2.0.0-rc3", email.subject);
    assert!(email.body_html.is_none());
    assert!(email.body_text.is_some());
}

#[test]
fn test_get_email_encoded_word_iso_8859_1() {
    let email_repository = MboxFile::new("datasets/test_lex.mbox").unwrap();
    let email = email_repository.get_email(&0).unwrap();
    assert_eq!("[l.educonnect.cp] ÉduConnect - Perturbation sur le service d'authentification responsables et élèves", email.subject);
    assert!(email.body_html.is_none());
    assert!(email.body_text.is_some());
}

#[test]
fn test_get_email_encoded_word_utf8() {
    let email_repository = MboxFile::new("datasets/test_lex.mbox").unwrap();
    let email = email_repository.get_email(&2).unwrap();
    assert_eq!("Modernisez vos processus RH sans complexité", email.subject);
    assert!(email.body_html.is_none());
    assert!(email.body_text.is_some());
}
