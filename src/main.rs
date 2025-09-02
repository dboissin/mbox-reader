use mbox_viewer::mailbox::{file::EmailFilePtr, FileSource, Mbox};

fn main() {
    let mailbox:Mbox<EmailFilePtr> = FileSource("fegge").try_into().unwrap();
}
