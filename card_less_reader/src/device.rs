use crate::error;
use crate::tlv_parser;

use error::*;
use std::sync::{atomic::AtomicBool, Arc};
use tlv_parser::Tlv;

pub trait CardLessDevice : Send {
    fn get_sn(&self) -> Result<String, DeviceError>;

    fn poll_emv(
        &self,
        purchase: Option<PollEmvPurchase>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<PollEmvResult, DeviceError>;

    
    fn ext_display_supported(&self) -> bool {
        false
    }

    fn get_ext_display_mode(&self) -> Result<ExtDisplayMode, DeviceError> {
        Err(DeviceError::NotSupported)
    }

    fn set_ext_display_mode(&self, _: &ExtDisplayMode) -> Result<(), DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

#[derive(Debug)]
pub struct PollEmvPurchase {
    pub p_type: u8,
    pub currency_code: u16,
    pub amount: u64,
}

impl PollEmvPurchase {
    pub fn new(p_type: u8, currency_code: u16, amount: u64) -> Self {
        Self {
            p_type,
            currency_code,
            amount,
        }
    }
}

pub enum PollEmvResult {
    Canceled,
    Success(Tlv),
}

#[derive(Copy, Clone)]
pub enum ExtDisplayMode {
    NoDisplay,
    Simple,
    Full,
}
