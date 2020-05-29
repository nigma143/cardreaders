use crate::tlv_parser;

use cancellation::OperationCanceled;
use thiserror::Error;
use tlv_parser::{Tlv, TlvError};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("operation canceled")]
    OperationCanceled,
    #[error("message channel error: {0}")]
    MessageChannel(String),
    #[error("TLV content error: {0}")]
    TlvContent(String, Tlv),
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum MessageChannelError {
    #[error("operation canceled")]
    OperationCanceled,
    #[error("{0}")]
    Other(String),
}

impl From<OperationCanceled> for MessageChannelError {
    fn from(_: OperationCanceled) -> Self {
        MessageChannelError::OperationCanceled
    }
}

impl From<MessageChannelError> for DeviceError {
    fn from(error: MessageChannelError) -> Self {
        DeviceError::MessageChannel(format!("{:?}", error))
    }
}

impl From<TlvError> for DeviceError {
    fn from(error: TlvError) -> Self {
        DeviceError::MessageChannel(format!("{:?}", error))
    }
}
