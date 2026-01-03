use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnnotError {
    #[error("IO error ({context}): {source}")]
    Io {
        #[source]
        source: std::io::Error,
        context: String,
    },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid input: {0}")]
    Validation(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Diff error: {0}")]
    Diff(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Window error: {0}")]
    Window(String),
}

impl AnnotError {
    pub fn io(source: std::io::Error, context: impl Into<String>) -> Self {
        Self::Io {
            source,
            context: context.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AnnotError>;
