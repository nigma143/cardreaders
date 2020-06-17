use card_less_reader::error::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WriteMessageError {
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum ReadMessageError {
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum TryReadMessageError {
    #[error("empty")]
    Empty,
    #[error("{0}")]
    Other(String),
}

impl From<WriteMessageError> for DeviceError {
    fn from(error: WriteMessageError) -> Self {
        match error {
            WriteMessageError::Other(m) => DeviceError::MessageChannel(m),
        }
    }
}

impl From<ReadMessageError> for DeviceError {
    fn from(error: ReadMessageError) -> Self {
        match error {
            ReadMessageError::Other(m) => DeviceError::MessageChannel(m),
        }
    }
}
