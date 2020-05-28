use std::ops::Deref;

use thiserror::Error;
use byteorder::{BigEndian, ByteOrder};

#[derive(Error, Debug)]
pub enum TagValueParseError {
    #[error("{0}")]
    Other(String),
}

impl From<std::string::FromUtf8Error> for TagValueParseError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        TagValueParseError::Other(format!("{:?}", error))
    }
}

pub trait TagValue {
    fn from_raw(raw: Vec<u8>) -> Result<Self, TagValueParseError> where Self: Sized;
    fn get_bytes() -> Vec<u8>;
}

pub struct StringAsciiTagValue {
    val: String,
}

impl StringAsciiTagValue {
    pub fn from(val: String) -> Self {
        Self { val }
    }

    pub fn from_raw(raw: Vec<u8>) -> Result<Self, TagValueParseError> {
        Ok(Self {
            val: String::from_utf8(raw)?
        })
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

impl U16BigEndianTagValue {
    pub fn from(val: u16) -> Self {
        Self { val }
    }

    pub fn from_raw(raw: Vec<u8>) -> Result<Self, TagValueParseError> {
        Ok(Self {
            val:  BigEndian::read_u16(&raw)
        })
    }
}

impl Deref for U16BigEndianTagValue {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.val
    }    
}

pub struct IntegerTagValue {
    val: i32,
    scale: u8
}

impl IntegerTagValue {
    pub fn from(val: i32, scale: u8) -> Self {
        Self { 
            val: val,
            scale: scale
        }
    }

    pub fn from_raw(raw: Vec<u8>) -> Result<Self, TagValueParseError> {    
                todo!()
            }
}

impl Deref for IntegerTagValue {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.val
    }    
}
