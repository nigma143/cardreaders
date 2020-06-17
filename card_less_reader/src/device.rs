use crate::error;
use crate::tlv_parser;

use error::*;
use std::sync::{atomic::AtomicBool, Arc};
use tlv_parser::Tlv;

pub trait CardLessDevice {
    fn get_sn(&self) -> Result<String, DeviceError>;

    fn get_ext_display_mode(&self) -> Result<ExtDisplayMode, DeviceError>;

    fn set_ext_display_mode(&self, value: &ExtDisplayMode) -> Result<(), DeviceError>;

    fn poll_emv(
        &self,
        purchase: Option<PollEmvPurchase>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<PollEmvResult, DeviceError>;
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
