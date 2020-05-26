use std::sync::mpsc::RecvTimeoutError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ByteChannelError {
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum MessageChannelError {
    #[error("byte channel error")]
    ByteChannel(#[from] ByteChannelError),
    #[error("invalid message request type")]
    InvalidRequestMessageType(),
    #[error("invalid message response type")]
    InvalidResponseMessageType(),
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum TlvChannelError {
    #[error("message channel error")]
    MessageChannel(#[from] MessageChannelError),
    #[error("tlv error: {0}")]
    TlvError(String),
}

#[derive(Error, Debug)]
pub enum TlvQueueError {
    #[error("tlv channel error: {0}")]
    TlvChannel(#[from] TlvChannelError),
    #[error("receive timeout error: {0}")]
    RecvTimeout(#[from] RecvTimeoutError),
    #[error("put error. return code: {0}")]
    PutError(u16),
}

#[derive(Error, Debug)]
pub enum GetResponseError {}

#[derive(Error, Debug)]
pub enum TlvValueParseError {
    #[error("many values")]
    ManyValues,
    #[error("{0}")]
    Other(String),
}

impl From<tlv_parser::TlvError> for TlvChannelError {
    fn from(error: tlv_parser::TlvError) -> Self {
        TlvChannelError::TlvError(format!("{}", error))
    }
}

impl From<TlvValueParseError> for TlvChannelError {
    fn from(error: TlvValueParseError) -> Self {
        TlvChannelError::TlvError(format!("{}", error))
    }
}
