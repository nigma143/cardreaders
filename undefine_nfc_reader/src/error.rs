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
    Other(String)
}

#[derive(Error, Debug)]
pub enum TlvChannelError {
    #[error("message channel error")]
    MessageChannel(#[from] MessageChannelError)
}


#[derive(Error, Debug)]
pub enum TlvValueParseError {
    #[error("many values")]
    ManyValues,
    #[error("{0}")]
    Other(String),
}
