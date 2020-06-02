use crate::error;
use crate::tlv_parser;

use cancellation::CancellationToken;

use error::*;
use tlv_parser::Tlv;

pub trait CardLessDevice {
    fn get_serial_number(&self) -> Result<String, DeviceError>;

    fn poll_emv(&self, ct: &CancellationToken) -> Result<PollEmvResult, DeviceError>;
}

pub struct PollEmvParameters {
    //Canceled,
}

pub enum PollEmvResult {
    Canceled,
    Success(Tlv),
}
