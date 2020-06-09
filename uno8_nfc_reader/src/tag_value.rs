use card_less_reader::tag_value::*;
use card_less_reader::{
    device::ExternalDisplayMode,
    tlv_parser::{TagValue, TlvError},
};
use std::ops::Deref;

pub struct SerialNumberTagValue {
    bom_version: U16BigEndianTagValue,
    partial_pn: U16BigEndianTagValue,
    unique_id: HexTagValue,
}

impl SerialNumberTagValue {
    pub fn get_bom_version(&self) -> u16 {
        *self.bom_version
    }

    pub fn get_partial_pn(&self) -> u16 {
        *self.partial_pn
    }

    pub fn get_unique_id(&self) -> String {
        self.unique_id.to_owned()
    }
}

impl TagValue for SerialNumberTagValue {
    type Value = (U16BigEndianTagValue, U16BigEndianTagValue, HexTagValue);

    fn new(val: Self::Value) -> Self {
        Self {
            bom_version: val.0,
            partial_pn: val.1,
            unique_id: val.2,
        }
    }

    fn from_raw(raw: &[u8]) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        Ok(Self {
            bom_version: U16BigEndianTagValue::from_raw(&raw[0..2])?,
            partial_pn: U16BigEndianTagValue::from_raw(&raw[2..4])?,
            unique_id: HexTagValue::from_raw(&raw[4..8])?,
        })
    }

    fn bytes(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.append(&mut self.bom_version.bytes());
        vec.append(&mut self.partial_pn.bytes());
        vec.append(&mut self.unique_id.bytes());
        vec
    }
}

pub struct ExternalDisplayModeTagValue {
    val: ExternalDisplayMode,
}

impl TagValue for ExternalDisplayModeTagValue {
    type Value = ExternalDisplayMode;

    fn new(val: Self::Value) -> Self {
        Self { val: val }
    }

    fn from_raw(raw: &[u8]) -> Result<Self, TlvError>
    where
        Self: Sized,
    {
        if raw.len() > 1 {
            return Err(TlvError::ParseTagValue(format!("expected 1 byte")));
        }

        Ok(Self {
            val: match raw.first() {
                Some(a) => match a {
                    0x00 => ExternalDisplayMode::NoExternalDisplay,
                    0x01 => ExternalDisplayMode::SendIndexOfPresetMessage,
                    0x02 => ExternalDisplayMode::SendFilteredPresetMessages,
                    _ => {
                        return Err(TlvError::ParseTagValue(format!(
                            "unknown ExternalDisplayMode code {:02X}",
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
            ExternalDisplayMode::NoExternalDisplay => [0x00].to_vec(),
            ExternalDisplayMode::SendIndexOfPresetMessage => [0x01].to_vec(),
            ExternalDisplayMode::SendFilteredPresetMessages => [0x02].to_vec(),
        }
    }
}

impl Deref for ExternalDisplayModeTagValue {
    type Target = ExternalDisplayMode;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
