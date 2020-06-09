use crate::error;
use crate::message_channel;
use crate::tag_value;

use std::time::Duration;

use cancellation::{CancellationToken, CancellationTokenSource};
use card_less_reader::{
    device::*,
    error::*,
    tag_value::{AnnexE, AnnexETagValue, IntTagValue, StringAsciiTagValue, U16BigEndianTagValue},
    tlv_parser::{TagValue, Tlv, Value},
};

use error::*;
use message_channel::{MessageChannel, ReadMessage, WriteMessage};
use tag_value::SerialNumberTagValue;

pub struct Uno8NfcDevice<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    channel: TMessageChannel,
    read_timeout: Duration,
    ask_timeout: Duration,

    external_display: Option<Box<dyn Fn(&String) + Send>>,
    internal_log: Option<Box<dyn Fn(&String) + Send>>,
    card_removal: Option<Box<dyn Fn() + Send>>,
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
            read_timeout: Duration::from_millis(1500),
            ask_timeout: Duration::from_millis(30),
            external_display: None,
            internal_log: None,
            card_removal: None,
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

    pub fn set_external_display(&mut self, f: impl Fn(&String) + Send + 'static) {
        self.external_display = Some(Box::new(f));
    }

    pub fn set_internal_log(&mut self, f: impl Fn(&String) + Send + 'static) {
        self.internal_log = Some(Box::new(f));
    }

    pub fn set_card_removal(&mut self, f: impl Fn() + Send + 'static) {
        self.card_removal = Some(Box::new(f));
    }

    fn write_do(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Do(tlv))
    }

    fn write_get(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Get(tlv))
    }

    fn write_set(&self, tlv: &Tlv) -> Result<(), DeviceError> {
        self.write(&WriteMessage::Set(tlv))
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

    fn read_timeout(&self) -> Result<Tlv, DeviceError> {
        let cts = CancellationTokenSource::new();
        cts.cancel_after(self.read_timeout);

        self.read(&cts)
    }

    fn read(&self, ct: &CancellationToken) -> Result<Tlv, DeviceError> {
        loop {
            let read_message = match self.channel.read(ct) {
                Ok(m) => m,
                Err(e) => match e {
                    ReadMessageError::OperationCanceled => {
                        return Err(DeviceError::Timeout(format!("response not received")))
                    }
                    ReadMessageError::Other(m) => return Err(DeviceError::MessageChannel(m)),
                },
            };

            let tlv = match read_message {
                ReadMessage::Ask => {
                    return Err(DeviceError::Other(format!(
                        "returned Ack message not expected"
                    )))
                }
                ReadMessage::Nack(code) => {
                    return Err(DeviceError::Other(format!(
                        "returned Nack({}) message not expected",
                        code
                    )))
                }
                ReadMessage::Do(tlv) => {
                    if let Some(display_message) =
                        tlv.get_val::<StringAsciiTagValue>("FF01 / DF46")?
                    {
                        if let Some(handler) = &self.external_display {
                            handler(&display_message)
                        }
                        continue;
                    }
                    if let Some(internal_log) =
                        tlv.get_val::<StringAsciiTagValue>("FF01 / DF8154")?
                    {
                        if let Some(handler) = &self.internal_log {
                            handler(&internal_log)
                        }
                        continue;
                    }
                    if let Some(_) = tlv.find_val("FF01 / DF08") {
                        if let Some(handler) = &self.card_removal {
                            handler()
                        }
                        continue;
                    }

                    tlv
                }
                ReadMessage::Get(tlv) => tlv,
                ReadMessage::Set(tlv) => tlv,
            };

            return Ok(tlv);
        }
    }

    fn read_timeout_success(&self) -> Result<Tlv, DeviceError> {
        let cts = CancellationTokenSource::new();
        cts.cancel_after(self.read_timeout);

        self.read_success(&cts)
    }

    fn read_success(&self, ct: &CancellationToken) -> Result<Tlv, DeviceError> {
        let tlv = self.read(ct)?;
        match tlv.tag() {
            0xFF01 => Ok(tlv),
            0xFF02 => Err(DeviceError::TlvContent(format!("Tag and length of Unsupported Instruction/s. Template contains chained tags and length of the instruction / s not supported by the PCD"), tlv)),
            0xFF03 => Err(DeviceError::TlvContent(format!("Tag and length of Failed Instruction/s. Template contains chained tags and length of the instruction / s that failed; an error number may be added"), tlv)),
            _ => Err(DeviceError::TlvContent(format!("Expected ResponseTemplates tag"), tlv))
        }
    }

    fn stop_macro(&self) -> Result<(), DeviceError> {
        self.write_do(&Tlv::new(0xDF7D, Value::Nothing)?)?;
        self.read_timeout_success()?;
        Ok(())
    }

    fn set_poll_timeout(&self, value: u16) -> Result<(), DeviceError> {
        self.write_do(&Tlv::new_spec(0xDF8212, U16BigEndianTagValue::new(value))?)?;
        self.read_timeout_success()?;
        Ok(())
    }

    pub fn set_external_display_mode(&self, value: ExternalDisplayMode) -> Result<(), DeviceError> {
        self.write_set(&Tlv::new(
            0xDF46,
            Value::Val(match value {
                ExternalDisplayMode::NoExternalDisplay => [0x00].to_vec(),
                ExternalDisplayMode::SendIndexOfPresetMessage => [0x01].to_vec(),
                ExternalDisplayMode::SendFilteredPresetMessages => [0x02].to_vec(),
            }),
        )?)?;
        self.read_timeout_success()?;
        Ok(())
    }
}

pub enum ExternalDisplayMode {
    NoExternalDisplay,
    SendIndexOfPresetMessage,
    SendFilteredPresetMessages,
}

impl<TMessageChannel> CardLessDevice for Uno8NfcDevice<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    fn get_serial_number(&self) -> Result<String, DeviceError> {
        self.write_get(&Tlv::new(0xDF4D, Value::Nothing)?)?;
        let tlv = self.read_timeout_success()?;
        match tlv.get_val::<SerialNumberTagValue>("FF01 / DF4D")? {
            Some(s) => Ok(format!(
                "{}_{}_{}",
                s.get_bom_version(),
                s.get_partial_pn(),
                s.get_unique_id()
            )),
            None => Err(DeviceError::TlvContent(
                format!("expected serial number tag"),
                tlv,
            )),
        }
    }

    fn poll_emv(
        &self,
        purchase: Option<PollEmvPurchase>,
        ct: &CancellationToken,
    ) -> Result<PollEmvResult, DeviceError> {
        self.set_poll_timeout(0)?;

        let r_tlv = match purchase {
            Some(s) => Tlv::new(
                0xFD,
                Value::TlvList(vec![
                    Tlv::new(0x9C, Value::Val(vec![s.p_type]))?,
                    Tlv::new_spec(0x5F2A, IntTagValue::new((s.currency_code as u64, 4)))?,
                    Tlv::new_spec(0x9F02, IntTagValue::new((s.amount, 12)))?,
                ]),
            )?,
            None => Tlv::new(0xFD, Value::Nothing)?,
        };

        self.write_do(&r_tlv)?;

        let dummy_ct = CancellationTokenSource::new();

        let mut current_ct = ct;
        loop {
            match self.read(current_ct) {
                Ok(tlv) => {
                    if let Some(terminate) = tlv.get_val::<AnnexETagValue>("FF03 / F2 / DF68")? {
                        match *terminate {
                            AnnexE::EmvTransactionTerminated => return Ok(PollEmvResult::Canceled),
                            AnnexE::CollisionMoreThanOnePICCDetected => continue,
                            AnnexE::EmvTransactionTerminatedSeePhone => continue,
                            AnnexE::EmvTransactionTerminatedUseContactChannel => continue,
                            AnnexE::EmvTransactionTerminatedTryAgain => continue,
                        }
                    }
                    if let Some(_) = tlv.find_val("FF01 / FC") {
                        return Ok(PollEmvResult::Success(tlv));
                    }

                    return Err(DeviceError::TlvContent(
                        format!("invalid response TLV"),
                        tlv,
                    ));
                }
                Err(e) => match e {
                    DeviceError::Timeout(m) => match ct.is_canceled() {
                        true => {
                            self.stop_macro()?;
                            current_ct = &dummy_ct;
                        }
                        false => return Err(DeviceError::Timeout(m)),
                    },
                    _ => return Err(e),
                },
            };
        }
    }
}
