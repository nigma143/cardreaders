use std::ops::{Deref, DerefMut};

use hidapi::*;
use tlv_parser::tlv::{Tlv, Value};
use undefine_nfc_reader::frame_channel::HidFrameChannel;
use undefine_nfc_reader::message_channel::{Message, MessageChannel};
use undefine_nfc_reader::tlv::{AsciiString, TlvDecorator, TlvExtensions};
use undefine_nfc_reader::tlv_channel::{ReadTlv, TlvChannel, WriteTlv};
use undefine_nfc_reader::tlv_queue::TlvQueue;

fn main() {
    simple_logger::init().unwrap();

    let hidapi = HidApi::new().unwrap();
    let device = hidapi.open(0x1089, 0x0001).unwrap();
    device.set_blocking_mode(false).unwrap();

    let frame_channel = HidFrameChannel::new(hidapi.open(0x1089, 0x0001).unwrap());
    let message_channel = MessageChannel::new(frame_channel);
    let tlv_channel = TlvChannel::new(message_channel);
    let tlv_queue = TlvQueue::new(tlv_channel);

    let m = WriteTlv::Get(Tlv::new_with_raw_val(0xDF46, vec![0x00, 0x00]).unwrap());

    tlv_queue.put(m).unwrap();

    let rs = tlv_queue
        .get(|x| true, std::time::Duration::from_millis(500))
        .unwrap();

    println!("{}", rs);

    drop(tlv_queue);

    println!("dsfsd");

    /*tlv_channel
        .write(&WriteTlv::Get(
            Tlv::new_with_raw_val(0xDF46, vec![0x00, 0x00]).unwrap(),
        ))
        .unwrap();

    let rs = tlv_channel.read().unwrap();
    println!("{}", rs);

    let rs = tlv_channel.read().unwrap();
    println!("{}", rs);

    let t1 = Tlv::new_with_raw_val(0xDF01, vec![0x00]).unwrap();

    let t2 = Tlv::new_with_childs(0xFF02, vec![t1]).unwrap();

    let t3 = Tlv::new_with_childs(0xFF03, vec![t2]).unwrap();

    println!("{}", TlvDecorator::new(&t3));*/
}
