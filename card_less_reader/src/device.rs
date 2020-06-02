use crate::error;
use crate::tlv_parser;

use cancellation::CancellationToken;

use error::*;
use tlv_parser::Tlv;

pub trait CardLessDevice {
    fn get_serial_number(&self) -> Result<String, DeviceError>;

    fn poll_emv(
        &self,
        purchase: Option<PollEmvPurchase>,
        ct: &CancellationToken,
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
