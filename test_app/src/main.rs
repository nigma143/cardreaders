use hidapi::*;
use std::fmt;
use tlv_parser::tlv::Tlv;
use undefine_nfc_reader::frame_channel::HidFrameChannel;
use undefine_nfc_reader::message_channel::{Message, MessageChannel};
use undefine_nfc_reader::tlv::TlvExtended;

fn main() {
    let hidapi = HidApi::new().unwrap();
    let device = hidapi.open(0x1089, 0x0001).unwrap();
    device.set_blocking_mode(false).unwrap();

    let frame_channel = HidFrameChannel::new(hidapi.open(0x1089, 0x0001).unwrap());
    let message_channel = MessageChannel::new(frame_channel);

    let mut rq = Message::Get {
        payload: vec![0xDF, 0x46, 0x00],
    };

    message_channel.write(&mut rq).unwrap();

    let rsAsk = message_channel.read().unwrap();
    let rs = message_channel.read().unwrap();

    let payload = match rs {
        Message::Do { payload } => payload,
        Message::Get { payload } => payload,
        Message::Set { payload } => payload,
        _ => panic!(":dsfdsfdsf"),
    };

    let tlv = Tlv::from_vec(&payload).unwrap();

    println!("{}", TlvExtended::new(tlv));
}
