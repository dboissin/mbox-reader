use crate::Email;

pub enum SearchError {
    ModelNotFound,
    Error
}

pub trait MailSearchRepository {
    type EmailId;

    fn index(&self, id: &Self::EmailId, email: &Email) -> Result<(), SearchError>;

    fn search(&self, content: &str) -> Result<Self::EmailId, SearchError>;

}
