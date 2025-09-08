pub mod mailbox;
pub mod embedding;
pub mod search;
pub mod storage;

pub use mailbox::Email;
pub use search::MailSearchRepository;
pub use search::SearchResult;
pub use storage::MailStorageRepository;
