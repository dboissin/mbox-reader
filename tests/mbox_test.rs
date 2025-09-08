use mbox_viewer::{embedding::{local::InternalEmbedder, Embedder}, mailbox::MailboxService, storage::{file::MboxFile, MailboxError}, MailStorageRepository};



#[test]
fn test_not_exists_mbox_file() {
    let mbox:Result<MboxFile,MailboxError> = MboxFile::new("/chemin/vers/fichier/inexistant");
    assert!(mbox.is_err());
    assert!(mbox.as_ref().is_err_and(|e| *e == MailboxError::MboxFileNotFound));
    assert!(mbox.as_ref().is_err_and(|e| *e != MailboxError::MboxParseError));
}

#[test]
fn test_exists_mbox_file() {
    let mbox:Result<MboxFile,MailboxError> = MboxFile::new("datasets/dev_apisix_apache_org.mbox");
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

#[test]
fn test_embed_sentences() {
    let embedder = InternalEmbedder::new()
        .unwrap_or_else(|_| panic!("Embedder init error"));
    let sentences = [
        "Modernisez vos processus RH sans complexité",
        "[l.educonnect.cp] ÉduConnect - Perturbation sur le service d'authentification responsables et élèves"
    ];
    let res = embedder.embed(&sentences);
    assert!(res.is_ok());
    assert_eq!(2, res.unwrap().len());
}

#[test]
fn test_embed_sentence() {
    let embedder = InternalEmbedder::new()
        .unwrap_or_else(|_| panic!("Embedder init error"));
    let sentence = "Modernisez vos processus RH sans complexité";
    let res = embedder.embed_line(sentence);
    assert!(res.is_ok());
    assert_eq!(384, res.unwrap().len());
}

#[test]
fn test_index_and_search_memory_cos() {
    let mut mailbox_service:MailboxService<MboxFile> = "datasets/test_emails_1000.mbox".try_into().unwrap();
    mailbox_service.index_emails();
    let emails = mailbox_service.search_email("Je cherche un email en rapport avec une mise à jour de logiciel.").unwrap();
    assert_eq!(5, emails.len());
    for (_, email) in &emails {
        assert!(email.body_text.as_ref().unwrap().contains("logiciel"))
    }
}
