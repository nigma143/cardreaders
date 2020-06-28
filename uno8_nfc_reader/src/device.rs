use crate::error;
use crate::message_channel;
use crate::tag_value;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::{thread, time::Duration};

use card_less_reader::{
    device::*,
    error::*,
    tag_value::{AnnexE, AnnexETagValue, IntTagValue, StringAsciiTagValue, U16BigEndianTagValue},
    tlv_parser::{TagValue, Tlv, Value},
};

use error::*;
use message_channel::{MessageChannel, ReadMessage, WriteMessage};
use tag_value::{ExtDisplayModeTagValue, SerialNumberTagValue};

struct NotifyCallbacks {
    external_display: Option<Box<dyn Fn(&String) + Send>>,
    internal_log: Option<Box<dyn Fn(&String) + Send>>,
    card_removal: Option<Box<dyn Fn() + Send>>,
}

pub struct Uno8NfcDevice {
    write_in: Sender<(WriteMessage, Sender<Result<(), WriteMessageError>>)>,
    read_out: Receiver<Result<ReadMessage, ReadMessageError>>,

    write_timeout: Duration,
    read_timeout: Duration,
    ask_timeout: Duration,

    notify_callbacks: Arc<Mutex<NotifyCallbacks>>,
}

impl Uno8NfcDevice {
    pub fn new(channel: impl MessageChannel + Send + 'static) -> Self {
        let (write_in_tx, write_in_rx) = mpsc::channel();
        let (read_out_tx, read_out_rx) = mpsc::channel();

        let notify_callbacks = Arc::new(Mutex::new(NotifyCallbacks {
            external_display: None,
            internal_log: None,
            card_removal: None,
        }));

        let notify_callbacks_ref = notify_callbacks.clone();

        thread::spawn(move || {
            Self::channel_loop(channel, notify_callbacks_ref, write_in_rx, read_out_tx)
        });

        Self {
            write_in: write_in_tx,
            read_out: read_out_rx,

            write_timeout: Duration::from_millis(30),
            read_timeout: Duration::from_millis(1500),
            ask_timeout: Duration::from_millis(30),

            notify_callbacks: notify_callbacks,
        }
    }

    fn channel_loop(
        channel: impl MessageChannel,
        notify_callbacks: Arc<Mutex<NotifyCallbacks>>,
        write_in: Receiver<(WriteMessage, Sender<Result<(), WriteMessageError>>)>,
        read_out: Sender<Result<ReadMessage, ReadMessageError>>,
    ) {
        loop {
            match write_in.try_recv() {
                Ok((m, tx)) => match tx.send(channel.write(&m)) {
                    Ok(_) => {}
                    Err(_) => {
                        log::debug!("write_in receiver is disconnected");
                        break;
                    }
                },
                Err(e) => match e {
                    TryRecvError::Empty => match channel.try_read() {
                        Ok(o) => {
                            match &o {
                                ReadMessage::Do(tlv) => {
                                    if let Some(display_message) =
                                        tlv.get_val::<StringAsciiTagValue>("FF01 / DF46").unwrap()
                                    {
                                        if let Some(handler) =
                                            &notify_callbacks.lock().unwrap().external_display
                                        {
                                            handler(&display_message)
                                        }
                                        continue;
                                    }
                                    if let Some(internal_log) =
                                        tlv.get_val::<StringAsciiTagValue>("FF01 / DF8154").unwrap()
                                    {
                                        if let Some(handler) =
                                            &notify_callbacks.lock().unwrap().internal_log
                                        {
                                            handler(&internal_log)
                                        }
                                        continue;
                                    }
                                    if let Some(_) = tlv.find_val("FF01 / DF08") {
                                        if let Some(handler) =
                                            &notify_callbacks.lock().unwrap().card_removal
                                        {
                                            handler()
                                        }
                                        continue;
                                    }
                                }
                                _ => {}
                            }

                            match read_out.send(Ok(o)) {
                                Ok(_) => {}
                                Err(_) => {
                                    log::debug!("read_out receiver is disconnected");
                                    break;
                                }
                            }
                        }
                        Err(e) => match e {
                            TryReadMessageError::Empty => {}
                            TryReadMessageError::Other(m) => {
                                match read_out.send(Err(ReadMessageError::Other(m))) {
                                    Ok(_) => {}
                                    Err(_) => {
                                        log::debug!("read_out receiver is disconnected");
                                        break;
                                    }
                                }
                            }
                        },
                    },
                    TryRecvError::Disconnected => {
                        log::debug!("write_in sender is disconnected");
                        break;
                    }
                },
            }

            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Uno8NfcDevice {
    pub fn get_write_timeout(&self) -> Duration {
        self.write_timeout
    }

    pub fn set_write_timeout(&mut self, timeout: Duration) {
        self.write_timeout = timeout;
    }

    pub fn get_read_timeout(&self) -> Duration {
        self.read_timeout
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
        self.notify_callbacks.lock().unwrap().external_display = Some(Box::new(f));
    }

    pub fn set_internal_log(&mut self, f: impl Fn(&String) + Send + 'static) {
        self.notify_callbacks.lock().unwrap().internal_log = Some(Box::new(f));
    }

    pub fn set_card_removal(&mut self, f: impl Fn() + Send + 'static) {
        self.notify_callbacks.lock().unwrap().card_removal = Some(Box::new(f));
    }
}

impl Uno8NfcDevice {
    fn write_do(&self, tlv: Tlv) -> Result<(), DeviceError> {
        self.write(WriteMessage::Do(tlv))
    }

    fn write_get(&self, tlv: Tlv) -> Result<(), DeviceError> {
        self.write(WriteMessage::Get(tlv))
    }

    fn write_set(&self, tlv: Tlv) -> Result<(), DeviceError> {
        self.write(WriteMessage::Set(tlv))
    }

    fn write(&self, message: WriteMessage) -> Result<(), DeviceError> {
        let (w_tx, w_rx) = mpsc::channel();

        self.write_in
            .send((message, w_tx))
            .map_err(|_| DeviceError::MessageChannel("send write message fail".into()))?;

        match w_rx.recv_timeout(self.write_timeout) {
            Ok(o) => o?,
            Err(e) => match e {
                mpsc::RecvTimeoutError::Timeout => {
                    Err(DeviceError::Timeout("recieved write result".into()))?
                }
                mpsc::RecvTimeoutError::Disconnected => Err(DeviceError::MessageChannel(
                    "channel is disconnected".into(),
                ))?,
            },
        }

        let message = (match self.read_out.recv_timeout(self.ask_timeout) {
            Ok(o) => o,
            Err(e) => match e {
                mpsc::RecvTimeoutError::Timeout => {
                    Err(DeviceError::Timeout("recieved read ACK".into()))?
                }
                mpsc::RecvTimeoutError::Disconnected => Err(DeviceError::MessageChannel(
                    "channel is disconnected".into(),
                ))?,
            },
        })?;

        match message {
            ReadMessage::Ask => Ok(()),
            ReadMessage::Nack(code) => Err(DeviceError::Other(format!("returned Nack: {}", code))),
            ReadMessage::Do(_) => Err(DeviceError::Other(
                "returned Do message not expected".into(),
            )),
            ReadMessage::Get(_) => Err(DeviceError::Other(
                "returned Get message not expected".into(),
            )),
            ReadMessage::Set(_) => Err(DeviceError::Other(
                "returned Set message not expected".into(),
            )),
        }
    }

    fn read_success(&self) -> Result<Tlv, DeviceError> {
        let tlv = self.read()?;
        match tlv.tag() {
            0xFF01 => Ok(tlv),
            0xFF02 => Err(DeviceError::TlvContent("Tag and length of Unsupported Instruction/s. Template contains chained tags and length of the instruction / s not supported by the PCD".into(), tlv)),
            0xFF03 => Err(DeviceError::TlvContent("Tag and length of Failed Instruction/s. Template contains chained tags and length of the instruction / s that failed; an error number may be added".into(), tlv)),
            _ => Err(DeviceError::TlvContent("Expected ResponseTemplates tag".into(), tlv))
        }
    }

    fn read(&self) -> Result<Tlv, DeviceError> {
        let message = (match self.read_out.recv_timeout(self.read_timeout) {
            Ok(o) => o,
            Err(e) => match e {
                mpsc::RecvTimeoutError::Timeout => {
                    Err(DeviceError::Timeout("recieved read".into()))?
                }
                mpsc::RecvTimeoutError::Disconnected => Err(DeviceError::MessageChannel(
                    "channel is disconnected".into(),
                ))?,
            },
        })?;

        let tlv = match message {
            ReadMessage::Ask => {
                return Err(DeviceError::Other(
                    "returned Ack message not expected".into(),
                ))
            }
            ReadMessage::Nack(code) => {
                return Err(DeviceError::Other(format!(
                    "returned Nack({}) message not expected",
                    code
                )))
            }
            ReadMessage::Do(tlv) | ReadMessage::Get(tlv) | ReadMessage::Set(tlv) => tlv,
        };

        return Ok(tlv);
    }

    fn read_ct(&self, cancel_flag: Arc<AtomicBool>) -> Result<Tlv, DeviceError> {
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                Err(DeviceError::OperationCanceled)?
            }

            let message = (match self.read_out.try_recv() {
                Ok(o) => o,
                Err(e) => match e {
                    TryRecvError::Empty => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    TryRecvError::Disconnected => Err(DeviceError::MessageChannel(
                        "channel is disconnected".into(),
                    ))?,
                },
            })?;

            let tlv = match message {
                ReadMessage::Ask => Err(DeviceError::Other(
                    "returned Ack message not expected".into(),
                ))?,
                ReadMessage::Nack(code) => Err(DeviceError::Other(format!(
                    "returned Nack({}) message not expected",
                    code
                )))?,
                ReadMessage::Do(tlv) | ReadMessage::Get(tlv) | ReadMessage::Set(tlv) => tlv,
            };

            return Ok(tlv);
        }
    }
}

impl Uno8NfcDevice {
    fn stop_macro(&self) -> Result<(), DeviceError> {
        self.write_do(Tlv::new(0xDF7D, Value::Nothing)?)?;
        self.read_success()?;
        Ok(())
    }

    fn set_poll_timeout(&self, value: u16) -> Result<(), DeviceError> {
        self.write_do(Tlv::new_spec(0xDF8212, U16BigEndianTagValue::new(value))?)?;
        self.read_success()?;
        Ok(())
    }
}

impl CardLessDevice for Uno8NfcDevice {
    fn get_sn(&self) -> Result<String, DeviceError> {
        self.write_get(Tlv::new(0xDF4D, Value::Nothing)?)?;
        let tlv = self.read_success()?;
        match tlv.get_val::<SerialNumberTagValue>("FF01 / DF4D")? {
            Some(s) => Ok(format!(
                "{}_{}_{}",
                s.get_bom_version(),
                s.get_partial_pn(),
                s.get_unique_id()
            )),
            None => Err(DeviceError::TlvContent(
                "expected serial number tag".into(),
                tlv,
            )),
        }
    }

    fn poll_emv(
        &self,
        purchase: Option<PollEmvPurchase>,
        cancel_flag: Arc<AtomicBool>,
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

        self.write_do(r_tlv)?;

        let dummy_ct = Arc::new(AtomicBool::new(false));

        let mut current_ct = cancel_flag;
        loop {
            match self.read_ct(current_ct.clone()) {
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

                    Err(DeviceError::TlvContent("invalid response TLV".into(), tlv))?;
                }
                Err(e) => match e {
                    DeviceError::OperationCanceled => {
                        self.stop_macro()?;
                        current_ct = dummy_ct.clone()
                    }
                    _ => Err(e)?,
                },
            };
        }
    }
/*
    fn ext_dysplay(&self) -> Option<& dyn ExtDisplay> {
        Some(self)
    }*/
}

impl ExtDisplay for Uno8NfcDevice {
    fn get_display_mode(&self) -> Result<ExtDisplayMode, DeviceError> {
        self.write_get(Tlv::new(0xDF46, Value::Nothing)?)?;

        let tlv = self.read_success()?;
        match tlv.get_val::<ExtDisplayModeTagValue>("FF01 / DF46")? {
            Some(s) => Ok(*s),
            None => Err(DeviceError::TlvContent(
                "expected external display mode tag".into(),
                tlv,
            )),
        }
    }

    fn set_display_mode(&self, value: &ExtDisplayMode) -> Result<(), DeviceError> {
        self.write_set(Tlv::new_spec(0xDF46, ExtDisplayModeTagValue::new(*value))?)?;
        self.read()?;
        Ok(())
    }
}
