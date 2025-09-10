
use std::env;

use mbox_viewer::{mailbox::MailboxService, storage::file::MboxFile};


fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();
    let args: Vec<String> = env::args().collect();
    let search_request = &args[1];
    let mbox_file_path = &args[2];
    let mut mailbox:MailboxService<MboxFile> = mbox_file_path.as_str().try_into()
            .expect("Error initializing mailbox service");
    mailbox.index_emails();
    if let Ok(search_results) = mailbox.search_email(search_request) {
        for (score, email) in &search_results {
            println!("Score : {score}");
            println!("{email}");
        }
    }
}
