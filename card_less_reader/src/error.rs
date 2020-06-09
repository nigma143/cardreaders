use crate::tlv_parser;

use cancellation::OperationCanceled;
use thiserror::Error;
use tlv_parser::{Tlv, TlvError};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("operation canceled")]
    OperationCanceled,
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("message channel error: {0}")]
    MessageChannel(String),
    #[error("TLV content error: {0}")]
    TlvContent(String, Tlv),
    #[error("{0}")]
    Other(String),
}

impl From<TlvError> for DeviceError {
    fn from(error: TlvError) -> Self {
        DeviceError::MessageChannel(format!("{:?}", error))
    }
}

impl From<OperationCanceled> for DeviceError {
    fn from(error: OperationCanceled) -> Self {
        DeviceError::OperationCanceled
    }
}
