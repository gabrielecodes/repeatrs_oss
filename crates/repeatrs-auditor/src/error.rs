pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Query error: {0}")]
    Query(#[from] sqlx::Error),

    #[error("An unknown error has occurred: {0}")]
    Unknown(String),
}
