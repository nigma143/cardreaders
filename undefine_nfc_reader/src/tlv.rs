use crate::error::TlvValueParseError;

use std::ops::{Deref, DerefMut};

use tlv_parser::tlv::{Tlv, Value};
pub trait TlvExtensions {
    fn new_ext<T>(tag: usize, val: T) -> Result<Self, tlv_parser::TlvError>
    where
        T: TlvValue<T>,
        Self: Sized;
    fn find_val_ext<T>(&self, path: &str) -> Result<Option<T>, TlvValueParseError>
    where
        T: TlvValue<T>;
    fn val_ext<T>(&self) -> Result<T, TlvValueParseError>
    where
        T: TlvValue<T>;
}

impl TlvExtensions for Tlv {
    fn new_ext<T>(tag: usize, val: T) -> Result<Self, tlv_parser::TlvError>
    where
        T: TlvValue<T>,
    {
        Tlv::new(tag, Value::Val(val.bytes()))
    }
    fn find_val_ext<T>(&self, path: &str) -> Result<Option<T>, TlvValueParseError>
    where
        T: TlvValue<T>,
    {
        match self.find_val(path) {
            Some(v) => match v {
                Value::TlvList(_) => Err(TlvValueParseError::ManyValues),
                Value::Val(b) => Ok(Some(T::from_raw(b)?)),
                Value::Nothing => Ok(Some(T::from_raw(&[0; 0])?)),
            },
            None => Ok(None),
        }
    }
    fn val_ext<T>(&self) -> Result<T, TlvValueParseError>
    where
        T: TlvValue<T>,
    {
        match self.val() {
            Value::TlvList(_) => Err(TlvValueParseError::ManyValues),
            Value::Val(b) => Ok(T::from_raw(b)?),
            Value::Nothing => Ok(T::from_raw(&[0; 0])?),
        }
    }
}

pub fn display_tlv(tlv: &Tlv) -> String {
    let mut output = "".to_owned();
    display_tlv_rec(tlv, &mut "".to_owned(), &mut output);
    output
}

fn display_tlv_rec(tlv: &Tlv, ident: &mut String, output: &mut String) {
    output.push_str(&format!("{}- {:02X}: ", &ident, tlv.tag()));
    match tlv.val() {
        tlv_parser::tlv::Value::Val(val) => output.push_str(&format!("{:02X?}", val)),
        tlv_parser::tlv::Value::TlvList(childs) => {
            output.push_str("\r\n");
            ident.push_str("  ");
            for child in childs {
                display_tlv_rec(&child, ident, output);
            }
        }
        tlv_parser::tlv::Value::Nothing => output.push_str(""),
    }
}

pub trait TlvValue<T> {
    fn from_raw(raw: &[u8]) -> Result<Self, TlvValueParseError>
    where
        Self: Sized;
    fn bytes(&self) -> Vec<u8>;
}

pub struct AsciiString {
    val: String,
}

impl AsciiString {
    pub fn new(val: String) -> Self {
        AsciiString { val }
    }
}

impl Deref for AsciiString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl DerefMut for AsciiString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

impl TlvValue<AsciiString> for AsciiString {
    fn from_raw(raw: &[u8]) -> Result<AsciiString, TlvValueParseError> {
        Ok(Self {
            val: String::from_utf8(raw.to_vec())
                .map_err(|x| TlvValueParseError::Other(format!("{}", x)))?,
        })
    }
    fn bytes(&self) -> Vec<u8> {
        self.val.bytes().collect()
    }
}
