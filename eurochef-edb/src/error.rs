use thiserror::Error;

#[derive(Error, Debug)]
pub enum EurochefError {
    #[error("Unsupported: {0}")]
    Unsupported(String),
}
