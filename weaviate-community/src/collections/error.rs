/// All custom errors
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WeaviateError>;

#[derive(Error, Debug)]
pub enum WeaviateError {
    #[error("Invalid query parameters passed: {0}")]
    Query(String),
    #[error("Not configured: {0}")]
    NotConfigured(String),
    #[error("Batch error: {0}")]
    Batch(String),
    #[error("Schema error: {0}")]
    Schema(String),
    #[error("Backup error: {0}")]
    Backup(String),
    #[error("GraphQL error: {0}")]
    GraphQL(String),
    #[error("Nodes error: {0}")]
    Nodes(String),
    #[error("Classification error: {0}")]
    Classification(String),
    #[error("Module error: {0}")]
    Module(String),
    #[error("Request error")]
    Request(#[from] reqwest::Error),
    #[error("URL parse error")]
    UrlParse(#[from] url::ParseError),
    #[error("JSON parse error")]
    JsonParse(#[from] serde_json::Error),
}
