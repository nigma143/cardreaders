use crate::tlv_parser;

use std::ops::Deref;

use byteorder::{BigEndian, ByteOrder};
use thiserror::Error;

use tlv_parser::{TagValue, TlvError};

#[derive(Error, Debug)]
pub enum TagValueParseError {
    #[error("{0}")]
    Other(String),
}

impl From<std::string::FromUtf8Error> for TlvError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        TlvError::ParseTagValue(format!("{:?}", error))
    }
}

impl From<std::num::ParseIntError> for TlvError {
    fn from(error: std::num::ParseIntError) -> Self {
        TlvError::ParseTagValue(format!("{:?}", error))
    }
}

pub struct StringAsciiTagValue {
    val: String,
}

impl TagValue for StringAsciiTagValue {
    type Value = String;

    fn new(val: Self::Value) -> Self {
        Self { val }
    }

    fn from_raw(raw: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        Ok(Self {
            val: String::from_utf8(raw)?,
        })
    }

    fn bytes(&self) -> Vec<u8> {
        self.val.bytes().collect()
    }
}

impl Deref for StringAsciiTagValue {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub struct U16BigEndianTagValue {
    val: u16,
}

impl TagValue for U16BigEndianTagValue {
    type Value = u16;

    fn new(val: Self::Value) -> Self {
        Self { val }
    }

    fn from_raw(raw: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        Ok(Self {
            val: BigEndian::read_u16(&raw),
        })
    }

    fn bytes(&self) -> Vec<u8> {
        let mut buf = [0; 2];
        BigEndian::write_u16(&mut buf, self.val);
        buf.to_vec()
    }
}

impl Deref for U16BigEndianTagValue {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub struct IntTagValue {
    val: u64,
    size: usize,
}

impl TagValue for IntTagValue {
    type Value = (u64, usize);

    fn new(val: Self::Value) -> Self {
        Self {
            val: val.0,
            size: val.1,
        }
    }

    fn from_raw(raw: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        let mut str = String::new();
        for b in raw {
            str.push_str(&format!("{:x}", b as u8))
        }

        Ok(Self {
            val: str.parse()?,
            size: str.len(),
        })
    }

    fn bytes(&self) -> Vec<u8> {
        let mut str = format!("{}", self.val);
        while str.len() < self.size {
            str.insert(0, '0');
        }

        if str.len() % 2 != 0 {
            str.insert(0, '0');
        }

        str.bytes()
            .map(|x| x - 48_u8)
            .collect::<Vec<u8>>()
            .chunks(2)
            .map(|x| (x[0] * 16) + x[1])
            .collect()
    }
}

impl Deref for IntTagValue {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub enum AnnexE {
    EmvTransactionTerminated = 0x09,
    CollisionMoreThanOnePICCDetected = 0x06,
    EmvTransactionTerminatedSeePhone = 0x29,
    EmvTransactionTerminatedUseContactChannel = 0x2A,
    EmvTransactionTerminatedTryAgain = 0x2B,
}

pub struct AnnexETagValue {
    val: AnnexE,
}

impl TagValue for AnnexETagValue {
    type Value = AnnexE;

    fn new(val: Self::Value) -> Self {
        Self { val }
    }

    fn from_raw(raw: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        if raw.len() > 1 {
            return Err(TlvError::ParseTagValue(format!("expected 1 byte")));
        }

        Ok(Self {
            val: match raw.first() {
                Some(a) => match a {
                    0x09 => AnnexE::EmvTransactionTerminated,
                    0x06 => AnnexE::CollisionMoreThanOnePICCDetected,
                    0x29 => AnnexE::EmvTransactionTerminatedSeePhone,
                    0x2A => AnnexE::EmvTransactionTerminatedUseContactChannel,
                    0x2B => AnnexE::EmvTransactionTerminatedTryAgain,
                    _ => {
                        return Err(TlvError::ParseTagValue(format!(
                            "unknown AnnexE code {:02X}",
                            a
                        )))
                    }
                },
                _ => return Err(TlvError::ParseTagValue(format!("expected 1 byte"))),
            },
        })
    }

    fn bytes(&self) -> Vec<u8> {
        match self.val {
            AnnexE::EmvTransactionTerminated => [0x09].to_vec(),
            AnnexE::CollisionMoreThanOnePICCDetected => [0x06].to_vec(),
            AnnexE::EmvTransactionTerminatedSeePhone => [0x29].to_vec(),
            AnnexE::EmvTransactionTerminatedUseContactChannel => [0x2A].to_vec(),
            AnnexE::EmvTransactionTerminatedTryAgain => [0x2B].to_vec(),
        }
    }
}

impl Deref for AnnexETagValue {
    type Target = AnnexE;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
