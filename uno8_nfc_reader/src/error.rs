use cancellation::OperationCanceled;
use card_less_reader::error::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WriteMessageError {
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum ReadMessageError {
    #[error("operation canceled")]
    OperationCanceled,
    #[error("{0}")]
    Other(String),
}

impl From<OperationCanceled> for ReadMessageError {
    fn from(_: OperationCanceled) -> Self {
        ReadMessageError::OperationCanceled
    }
}

impl From<WriteMessageError> for DeviceError {
    fn from(error: WriteMessageError) -> Self {
        match error {
            WriteMessageError::Other(m) => DeviceError::MessageChannel(m),
        }
    }
}
