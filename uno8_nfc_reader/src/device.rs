use crate::error;
use crate::message_channel;
use crate::tag_value;
use crate::tlv_parser;

use std::time::Duration;

use cancellation::{CancellationToken, CancellationTokenSource};

use error::*;
use message_channel::{MessageChannel, ReadMessage, WriteMessage};
use tag_value::{AnnexE, AnnexETagValue, U16BigEndianTagValue};
use tlv_parser::{TagValue, Tlv, Value};

pub struct Uno8NfcDevice<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    channel: TMessageChannel,
    read_timeout: Duration,
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
            read_timeout: Duration::from_millis(15000),
            ask_timeout: Duration::from_millis(30),
        }
    }

    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read_timeout = timeout;
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

        let read_message = match self.channel.read(&cts) {
            Ok(m) => m,
            Err(e) => match e {
                ReadMessageError::OperationCanceled => {
                    return Err(DeviceError::Timeout(format!("ack not received")))
                }
                ReadMessageError::Other(m) => return Err(DeviceError::MessageChannel(m)),
            },
        };

        match read_message {
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

    pub fn read_timeout(&self) -> Result<Tlv, DeviceError> {
        let cts = CancellationTokenSource::new();
        cts.cancel_after(self.read_timeout);

        self.read(&cts)
    }

    pub fn read(&self, ct: &CancellationToken) -> Result<Tlv, DeviceError> {
        let read_message = match self.channel.read(ct) {
            Ok(m) => m,
            Err(e) => match e {
                ReadMessageError::OperationCanceled => {
                    return Err(DeviceError::Timeout(format!("response not received")))
                }
                ReadMessageError::Other(m) => return Err(DeviceError::MessageChannel(m)),
            },
        };

        match read_message {
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

    pub fn read_timeout_success(&self) -> Result<Tlv, DeviceError> {
        let cts = CancellationTokenSource::new();
        cts.cancel_after(self.read_timeout);

        self.read_success(&cts)
    }

    pub fn read_success(&self, ct: &CancellationToken) -> Result<Tlv, DeviceError> {
        let tlv = self.read(ct)?;
        match tlv.tag() {
            0xFF01 => Ok(tlv),
            0xFF02 => Err(DeviceError::TlvContent(format!("Tag and length of Unsupported Instruction/s. Template contains chained tags and length of the instruction / s not supported by the PCD"), tlv)),
            0xFF03 => Err(DeviceError::TlvContent(format!("Tag and length of Failed Instruction/s. Template contains chained tags and length of the instruction / s that failed; an error number may be added"), tlv)),
            _ => Err(DeviceError::TlvContent(format!("Expected ResponseTemplates tag"), tlv))
        }
    }

    pub fn stop_macro(&self) -> Result<(), DeviceError> {
        self.write_do(&Tlv::new(0xDF7D, Value::Nothing)?)?;
        self.read_timeout_success()?;
        Ok(())
    }

    pub fn set_poll_timeout(&self, value: u16, ct: &CancellationToken) -> Result<(), DeviceError> {
        self.write_do(&Tlv::new_spec(0xDF8212, U16BigEndianTagValue::new(value))?)?;
        self.read_success(ct)?;
        Ok(())
    }

    pub fn poll_emv(&self, ct: &CancellationToken) -> Result<PollEmvResult, DeviceError> {
        self.set_poll_timeout(0, ct)?;

        self.write_do(&Tlv::new(0xFD, Value::Nothing)?)?;

        let unlim_ct = CancellationTokenSource::new();

        let mut fact_ct = ct;
        loop {
            match self.read(fact_ct) {
                Ok(tlv) => {
                    if let Some(tarminate) = tlv.get_val::<AnnexETagValue>("FF03 / F2 / DF68")? {
                        match *tarminate {
                            AnnexE::EmvTransactionTerminated => return Ok(PollEmvResult::Canceled),
                            AnnexE::CollisionMoreThanOnePICCDetected => {}
                            AnnexE::EmvTransactionTerminatedSeePhone => {}
                            AnnexE::EmvTransactionTerminatedUseContactChannel => {}
                            AnnexE::EmvTransactionTerminatedTryAgain => {}
                        }
                    }
                    if let Some(_) = tlv.find_val("FF01 / FC") {
                        return Ok(PollEmvResult::Success(tlv));
                    }

                    return Err(DeviceError::TlvContent(format!("invalid response TLV"), tlv));
                }
                Err(e) => match e {
                    DeviceError::Timeout(m) => match ct.is_canceled() {
                        true => { 
                            self.stop_macro()?;
                            fact_ct = &unlim_ct;
                        },
                        false => return Err(DeviceError::Timeout(m)),
                    },
                    _ => return Err(e),
                },
            };
        }
    }
}

pub struct PollEmvParameters {
    //Canceled,
}

pub enum PollEmvResult {
    Canceled,
    Success(Tlv),
}
