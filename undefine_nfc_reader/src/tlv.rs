use crate::error;

use std::fmt;
use std::ops::{Deref, DerefMut};

use tlv_parser::tlv::{Tlv, Value};
use tlv_parser::TlvError;

use error::*;

pub struct TlvDecorator<'a> {
    tlv: &'a Tlv,
}

impl<'a> TlvDecorator<'a> {
    pub fn new(tlv: &'a Tlv) -> Self {
        TlvDecorator { tlv }
    }

    fn display_write(tlv: &Tlv, ident: &mut String, output: &mut String) {
        output.push_str(&format!("{}- {:02X}: ", &ident, tlv.tag()));
        match tlv.val() {
            tlv_parser::tlv::Value::Val(val) => output.push_str(&format!("{:02X?}", val)),
            tlv_parser::tlv::Value::TlvList(childs) => {
                output.push_str("\n");
                ident.push_str("  ");
                for child in childs {
                    Self::display_write(&child, ident, output);
                }
            }
            tlv_parser::tlv::Value::Nothing => output.push_str(""),
        }
    }
}

impl<'a> fmt::Debug for TlvDecorator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("Tlv");
        builder.field("tag", &self.tlv.tag());

        match self.tlv.val() {
            Value::TlvList(childs) => {
                let tlv_vec = &childs
                    .iter()
                    .map(|x| TlvDecorator::new(x))
                    .collect::<Vec<TlvDecorator>>();
                builder.field("val", &tlv_vec);
            }
            Value::Val(val) => {
                builder.field("val", val);
            }
            Value::Nothing => {}
        }

        builder.finish()
    }
}

impl<'a> fmt::Display for TlvDecorator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        Self::display_write(self.tlv, &mut "".to_owned(), &mut output);
        f.write_str(&output)
    }
}

pub trait TlvExtensions {
    fn new_with_val<T>(tag: usize, val: T) -> Result<Self, TlvError>
    where
        T: TlvValue<T>,
        Self: Sized;

    fn new_with_childs(tag: usize, val: Vec<Self>) -> Result<Self, TlvError>
    where
        Self: Sized;

    fn new_with_raw_val(tag: usize, val: Vec<u8>) -> Result<Self, TlvError>
    where
        Self: Sized;

    fn find_val_ext<T>(&self, path: &str) -> Result<Option<T>, TlvValueParseError>
    where
        T: TlvValue<T>;

    fn val_ext<T>(&self) -> Result<T, TlvValueParseError>
    where
        T: TlvValue<T>;
}

impl TlvExtensions for Tlv {
    fn new_with_val<T>(tag: usize, val: T) -> Result<Self, TlvError>
    where
        T: TlvValue<T>,
    {
        Tlv::new(tag, Value::Val(val.bytes()))
    }

    fn new_with_childs(tag: usize, vals: Vec<Tlv>) -> Result<Self, TlvError> {
        Tlv::new(tag, Value::TlvList(vals))
    }

    fn new_with_raw_val(tag: usize, val: Vec<u8>) -> Result<Self, TlvError> {
        Tlv::new(tag, Value::Val(val))
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
        Self { val: val }
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
