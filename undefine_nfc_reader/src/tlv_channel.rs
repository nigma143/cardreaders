use crate::error;
use crate::message_channel;
use crate::tlv;

use std::fmt;

use tlv_parser::tlv::{Tlv, Value};

use error::*;
use message_channel::{FrameChannel, Message, MessageChannel};
use tlv::{AsciiString, TlvDecorator, TlvExtensions};

pub enum WriteTlv {
    Do(Tlv),
    Get(Tlv),
    Set(Tlv),
}

pub enum ReadTlv {
    Ack,
    Nack(u16),
    Tlv(Tlv),
}

impl fmt::Debug for WriteTlv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WriteTlv::Do(tlv) => f
                .debug_struct("{Do}Tlv")
                .field("val", &TlvDecorator::new(tlv))
                .finish(),
            WriteTlv::Get(tlv) => f
                .debug_struct("(Get)Tlv")
                .field("val", &TlvDecorator::new(tlv))
                .finish(),
            WriteTlv::Set(tlv) => f
                .debug_struct("(Set)Tlv")
                .field("val", &TlvDecorator::new(tlv))
                .finish(),
        }
    }
}

impl fmt::Display for WriteTlv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WriteTlv::Do(tlv) => {
                write!(f, "(Do)Tlv(")?;
                writeln!(f)?;
                fmt::Display::fmt(&TlvDecorator::new(tlv), f)?;
                write!(f, ")")
            }
            WriteTlv::Get(tlv) => {
                write!(f, "(Get)Tlv(")?;
                writeln!(f)?;
                fmt::Display::fmt(&TlvDecorator::new(tlv), f)?;
                write!(f, ")")
            }
            WriteTlv::Set(tlv) => {
                write!(f, "(Set)Tlv(")?;
                writeln!(f)?;
                fmt::Display::fmt(&TlvDecorator::new(tlv), f)?;
                write!(f, ")")
            }
        }
    }
}

impl fmt::Debug for ReadTlv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadTlv::Ack => write!(f, "Ack"),
            ReadTlv::Nack(code) => f.debug_struct("Nack").field("code", code).finish(),
            ReadTlv::Tlv(tlv) => f
                .debug_struct("Tlv")
                .field("val", &TlvDecorator::new(tlv))
                .finish(),
        }
    }
}

impl fmt::Display for ReadTlv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadTlv::Ack => write!(f, "Ack"),
            ReadTlv::Nack(code) => write!(f, "Nack(code: {})", code),
            ReadTlv::Tlv(tlv) => {
                write!(f, "Tlv(")?;
                writeln!(f)?;
                fmt::Display::fmt(&TlvDecorator::new(tlv), f)?;
                write!(f, ")")
            }
        }
    }
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
        log::info!("write {}", data);

        let message = match data {
            WriteTlv::Do(tlv) => Message::Do(tlv.to_vec()),
            WriteTlv::Get(tlv) => Message::Get(tlv.to_vec()),
            WriteTlv::Set(tlv) => Message::Set(tlv.to_vec()),
        };
        self.channel.write(&message)?;
        Ok(())
    }

    pub fn read(&self) -> Result<ReadTlv, TlvChannelError> {
        let message = self.channel.read()?;

        let result = match message {
            Message::Ask => Ok(ReadTlv::Ack),
            Message::Nack(code) => Ok(ReadTlv::Nack(code)),
            Message::Do(payload) => {
                let tlv = Tlv::from_vec(&payload)?;
                if let Value::TlvList(childs) = tlv.val() {
                    for child in childs {
                        match child.tag() {
                            0xDF46 => {
                                //ExternalDisplay
                                let val = child.val_ext::<AsciiString>()?;
                                println!("External display: {}", *val);
                                return self.read();
                            }
                            0xDF8154 => {
                                //InternalLog
                                let val = child.val_ext::<AsciiString>()?;
                                println!("Internal log: {}", *val);
                                return self.read();
                            }
                            0xDF08 => {
                                //CardRemoval
                                println!("Card removal");
                                return self.read();
                            }
                            _ => {}
                        }
                    }
                }
                Ok(ReadTlv::Tlv(tlv))
            }
            Message::Get(payload) => Ok(ReadTlv::Tlv(Tlv::from_vec(&payload)?)),
            Message::Set(payload) => Ok(ReadTlv::Tlv(Tlv::from_vec(&payload)?)),
        };

        match &result {
            Ok(o) => log::info!("read {}", o),
            Err(e) => log::info!("error on read: {:?}", e),
        }

        result
    }
}
