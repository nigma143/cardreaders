use crate::message_channel::{MessageChannel,Message, FrameChannel};
use crate::error::*;

use tlv_parser::tlv::{Tlv, Value};

pub enum WriteTlv {
    Do(Tlv),
    Get(Tlv),
    Set(Tlv),
}

pub enum ReadTlv {
    Ack,
    Nack,
    Tlv(Tlv)
}

pub struct TlvChannel<TFrameChannel>
where
    TFrameChannel: FrameChannel,
{
    channel: MessageChannel<TFrameChannel>,
}

impl<TFrameChannel> TlvChannel<TFrameChannel>
where
    TFrameChannel: FrameChannel,
{
    pub fn new(channel: MessageChannel<TFrameChannel>) -> Self {
        Self { channel }
    }

    pub fn write(&self, data: &WriteTlv) -> Result<(), TlvChannelError> {
        let message = match data {
            WriteTlv::Do(tlv) => Message::Do(tlv.to_vec()),
            WriteTlv::Get(tlv) => Message::Get(tlv.to_vec()),
            WriteTlv::Set(tlv) => Message::Set(tlv.to_vec()),
        };
        self.channel.write(&message)?;
        Ok(())
    }
}