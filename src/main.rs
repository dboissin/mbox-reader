use mbox_viewer::{storage::file::MboxFile, FileSource};


fn main() {
    let mailbox:MboxFile = FileSource("fegge").try_into().unwrap();
}
