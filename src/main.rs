use mbox_viewer::{mailbox::MailboxService, storage::file::MboxFile};


fn main() {
    let mailbox:MailboxService<MboxFile> = "fegge".try_into().unwrap();
}
