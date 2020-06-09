use crate::error;

use card_less_reader::tlv_parser::Tlv;
use error::*;

#[derive(Debug)]
pub enum WriteMessage {
    Do(Tlv),
    Get(Tlv),
    Set(Tlv),
}

#[derive(Debug)]
pub enum ReadMessage {
    Ask,
    Nack(u8),
    Do(Tlv),
    Get(Tlv),
    Set(Tlv),
}

pub trait MessageChannel {
    fn write(&self, message: &WriteMessage) -> Result<(), WriteMessageError>;
    fn try_read(&self) -> Result<ReadMessage, TryReadMessageError>;
}
