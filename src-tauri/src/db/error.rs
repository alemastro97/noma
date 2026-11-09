use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("migration failed: {0}")]
    Migration(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("invalid recurrence rule: {0}")]
    InvalidRecurrence(String),
    #[error("invalid value: {0}")]
    Invalid(String),
}

pub type DbResult<T> = Result<T, DbError>;
