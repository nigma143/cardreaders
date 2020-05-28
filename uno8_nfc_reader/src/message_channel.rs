use crate::error;

use cancellation::CancellationToken;

use error::*;

#[derive(Debug, Clone)]
pub enum WriteMessage {
    Do(Vec<u8>),
    Get(Vec<u8>),
    Set(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum ReadMessage {
    Ask,
    Nack(u8),
    Do(Vec<u8>),
    Get(Vec<u8>),
    Set(Vec<u8>),
}

pub trait MessageChannel {
    fn write(&self, message: &WriteMessage) -> Result<(), MessageChannelError>;
    fn read(&self, ct: &CancellationToken) -> Result<ReadMessage, MessageChannelError>;
}
