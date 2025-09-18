use thiserror::Error;

#[derive(Error, Debug)]
pub enum UltraError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Tree shaking error: {0}")]
    TreeShaking(String),

    #[error("CSS processing error: {0}")]
    CssProcessing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, UltraError>;

impl From<regex::Error> for UltraError {
    fn from(err: regex::Error) -> Self {
        UltraError::Parse(format!("Regex error: {}", err))
    }
}

impl From<anyhow::Error> for UltraError {
    fn from(err: anyhow::Error) -> Self {
        UltraError::Build(err.to_string())
    }
}