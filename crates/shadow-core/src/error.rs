use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShadowError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("VMM error: {0}")]
    Vmm(String),

    #[error("VSOCK error: {0}")]
    Vsock(String),

    #[error("Filesystem error: {0}")]
    Filesystem(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout error: operation exceeded {0} ms")]
    Timeout(u64),

    #[error("Execution failed with exit code: {0}")]
    ExecutionFailed(i32),
}

pub type Result<T> = std::result::Result<T, ShadowError>;
