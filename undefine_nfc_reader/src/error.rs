use cancellation::OperationCanceled;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ByteChannelError {
    #[error("operation canceled")]
    OperationCanceled(#[from] OperationCanceled),
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum MessageChannelError {
    #[error("operation canceled")]
    OperationCanceled(#[from] OperationCanceled),
    #[error("{0}")]
    Other(String),
}

impl From<ByteChannelError> for MessageChannelError {
    fn from(error: ByteChannelError) -> Self {
        match error {
            ByteChannelError::OperationCanceled(c) => MessageChannelError::OperationCanceled(c),
            ByteChannelError::Other(m) => MessageChannelError::Other(m),
        }
    }
}