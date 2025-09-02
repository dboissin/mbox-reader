
use mbox_viewer::mailbox::{file::EmailFilePtr, FileSource, MailboxError, Mbox};

#[test]
fn test_not_exists_mbox_file() {
    let mbox:Result<Mbox<EmailFilePtr>,MailboxError> = FileSource("/chemin/vers/fichier/inexistant").try_into();
    assert!(mbox.is_err());
    assert!(mbox.as_ref().is_err_and(|e| *e == MailboxError::MboxFileNotFound));
    assert!(mbox.as_ref().is_err_and(|e| *e != MailboxError::MboxParseError));
}

#[test]
fn test_exists_mbox_file() {
    let mbox:Result<Mbox<EmailFilePtr>,MailboxError> = FileSource("datasets/dev_apisix_apache_org.mbox").try_into();
    assert!(mbox.is_ok());
}
