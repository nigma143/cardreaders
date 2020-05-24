use std::ops::{Deref, DerefMut};

use hidapi::*;
use tlv_parser::tlv::{Tlv, Value};
use undefine_nfc_reader::frame_channel::HidFrameChannel;
use undefine_nfc_reader::message_channel::{Message, MessageChannel};
use undefine_nfc_reader::tlv::{display_tlv, AsciiString, TlvExtensions, TlvValue};

fn main() {
    simple_logger::init().unwrap();

    let hidapi = HidApi::new().unwrap();
    let device = hidapi.open(0x1089, 0x0001).unwrap();
    device.set_blocking_mode(false).unwrap();

    let frame_channel = HidFrameChannel::new(hidapi.open(0x1089, 0x0001).unwrap());
    let message_channel = MessageChannel::new(frame_channel);

    let mut rq = Message::Get(vec![0xDF, 0x46, 0x00]);

    message_channel.write(&mut rq).unwrap();

    let rsAsk = message_channel.read().unwrap();
    let rs = message_channel.read().unwrap();

    let payload = match rs {
        Message::Do(payload) => payload,
        Message::Get(payload)  => payload,
        Message::Set(payload)  => payload,
        _ => panic!(":dsfdsfdsf"),
    };

    let tlv = Tlv::from_vec(&payload).unwrap();

    println!("{}", display_tlv(&tlv));

    let v: AsciiString = tlv.find_val_ext("FF01 / DF46").unwrap().unwrap();
    println!("{}", *v);

    let n_tlv = Tlv::new_with_childs(
        0xFF01,
        vec![Tlv::new_with_val(0x0C, AsciiString::new("Hui".to_owned())).unwrap()],
    )
    .unwrap();

    println!("{}", display_tlv(&n_tlv));

    let v: AsciiString = n_tlv.find_val_ext("FF01 / 0C").unwrap().unwrap();
    println!("{}", *v);
}
