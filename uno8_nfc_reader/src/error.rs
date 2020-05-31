use crate::tlv_parser;

use cancellation::OperationCanceled;
use thiserror::Error;
use tlv_parser::{Tlv, TlvError};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("message channel error: {0}")]
    MessageChannel(String),
    #[error("TLV content error: {0}")]
    TlvContent(String, Tlv),
    #[error("{0}")]
    Other(String),
}

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
            WriteMessageError::Other(m) => DeviceError::MessageChannel(m)
        }
    }
}

impl From<TlvError> for DeviceError {
    fn from(error: TlvError) -> Self {
        DeviceError::MessageChannel(format!("{:?}", error))
    }
}
