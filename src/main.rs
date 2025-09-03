use mbox_viewer::mailbox::{file::MboxFile, FileSource};

fn main() {
    let mailbox:MboxFile = FileSource("fegge").try_into().unwrap();
}
