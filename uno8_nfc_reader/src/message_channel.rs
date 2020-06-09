use crate::error;

use cancellation::CancellationToken;

use card_less_reader::tlv_parser::Tlv;
use error::*;

#[derive(Debug, Clone)]
pub enum WriteMessage<'a> {
    Do(&'a Tlv),
    Get(&'a Tlv),
    Set(&'a Tlv),
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
    fn read(&self, ct: &CancellationToken) -> Result<ReadMessage, ReadMessageError>;
}
