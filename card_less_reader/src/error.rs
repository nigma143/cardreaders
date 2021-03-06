use crate::tlv_parser;

use thiserror::Error;
use tlv_parser::{Tlv, TlvError};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("not supported")]
    NotSupported,
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

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("device error")]
    Device(#[from] DeviceError),
}
