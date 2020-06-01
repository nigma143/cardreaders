use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::thread;

use hidapi::*;
/*use tlv_parser::tlv::{Tlv, Value};
use undefine_nfc_reader::frame_channel::HidFrameChannel;
use undefine_nfc_reader::message_channel::{Message, MessageChannel};
use undefine_nfc_reader::tlv::{AsciiString, TlvDecorator, TlvExtensions};
use undefine_nfc_reader::tlv_channel::{ReadTlv, TlvChannel, WriteTlv};
use undefine_nfc_reader::tlv_handler::TlvHandler;*/

use cancellation::{CancellationTokenSource, OperationCanceled};

use card_less_reader::device::*;
use card_less_reader::tag_value::*;
use card_less_reader::tlv_parser::*;
use uno8_nfc_reader::device::{ExternalDisplayMode, Uno8NfcDevice};
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;
use uno8_nfc_reader::message_channel::{MessageChannel, ReadMessage, WriteMessage};

fn main() {
    simple_logger::init().unwrap();

    let device = Uno8NfcDeviceBuilder::use_hid(0x1089, 0x0001)
        .unwrap()
        .set_external_display(Box::new(|x| println!("Display: {}", x)))
        .set_internal_log(Box::new(|x| println!("Log: {}", x)))
        .set_card_removal(Box::new(|| println!("Card removal")))
        .finish();

    device
        .set_external_display_mode(ExternalDisplayMode::SendFilteredPresetMessages)
        .unwrap();

    poll_emv(&device);
}

fn poll_emv(device: &impl CardLessDevice) {
    let cts = CancellationTokenSource::new();
    //cts.cancel_after(std::time::Duration::from_millis(1500));

    let tt = device.poll_emv(&cts).unwrap();

    match tt {
        PollEmvResult::Canceled => println!("canceled"),
        PollEmvResult::Success(x) => println!("{}", x),
    }
}
