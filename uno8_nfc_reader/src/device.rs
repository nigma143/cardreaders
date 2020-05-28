use crate::error;
use crate::message_channel;
use crate::tlv_parser;

use std::time::Duration;

use cancellation::{CancellationToken, CancellationTokenSource};

use error::*;
use message_channel::{MessageChannel, ReadMessage, WriteMessage};
use tlv_parser::Tlv;

pub struct Uno8NfcDevice<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    channel: TMessageChannel,
    ask_timeout: Duration,
}

impl<TMessageChannel> Uno8NfcDevice<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    pub fn new(channel: TMessageChannel) -> Self
    where
        TMessageChannel: MessageChannel,
    {
        Self {
            channel: channel,
            ask_timeout: Duration::from_millis(150),
        }
    }

    pub fn set_ack_timeout(&mut self, timeout: Duration) {
        self.ask_timeout = timeout;
    }

    pub fn get_ack_timeout(&self) -> Duration {
        self.ask_timeout
    }

    pub fn write_do(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Do(tlv.to_vec()))
    }

    pub fn write_get(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Get(tlv.to_vec()))
    }

    pub fn write_set(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Set(tlv.to_vec()))
    }

    fn write(&self, message: &WriteMessage) -> Result<(), DeviceError> {
        self.channel.write(message)?;

        let cts = CancellationTokenSource::new();
        cts.cancel_after(self.ask_timeout);

        match self.channel.read(&cts)? {
            ReadMessage::Ask => Ok(()),
            ReadMessage::Nack(code) => Err(DeviceError::Other(format!("returned Nack: {}", code))),
            ReadMessage::Do(_) => Err(DeviceError::Other(format!(
                "returned Do message not expected"
            ))),
            ReadMessage::Get(_) => Err(DeviceError::Other(format!(
                "returned Get message not expected"
            ))),
            ReadMessage::Set(_) => Err(DeviceError::Other(format!(
                "returned Set message not expected"
            ))),
        }
    }

    pub fn read(&self, ct: &CancellationToken) -> Result<Tlv, DeviceError> {
        match self.channel.read(ct)? {
            ReadMessage::Ask => Err(DeviceError::Other(format!(
                "returned Ack message not expected"
            ))),
            ReadMessage::Nack(code) => Err(DeviceError::Other(format!(
                "returned Nack({}) message not expected",
                code
            ))),
            ReadMessage::Do(tlv_raw) => Ok(Tlv::from_vec(&tlv_raw)?),
            ReadMessage::Get(tlv_raw) => Ok(Tlv::from_vec(&tlv_raw)?),
            ReadMessage::Set(tlv_raw) => Ok(Tlv::from_vec(&tlv_raw)?),
        }
    }
}
