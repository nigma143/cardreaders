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

use uno8_nfc_reader::tlv_parser::{Tlv, Value};
use uno8_nfc_reader::device::Uno8NfcDevice;
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;
use uno8_nfc_reader::message_channel::{MessageChannel, ReadMessage, WriteMessage};

fn main() {
    simple_logger::init().unwrap();

    let device = Uno8NfcDeviceBuilder::use_hid(0x1089, 0x0001)
        .unwrap()
        .finish();

    //let m = Tlv::new_with_raw_val(0xDF46, vec![0x00, 0x00]).unwrap();

    //device.write_get(&m).unwrap();
    //device.read(&CancellationTokenSource::new()).unwrap();

    //MessageChannel::write(&device, &WriteMessage::Get(m.to_vec()), &CancellationTokenSource::new()).unwrap();

    //let r = MessageChannel::read(&device, &CancellationTokenSource::new()).unwrap();

    let cts = CancellationTokenSource::new();
    //cts.cancel_after(std::time::Duration::from_millis(1500));

    //let r2 = MessageChannel::read(&device, &cts).unwrap();
    /*
        device.set_blocking_mode(false).unwrap();

        let h = TlvHandler::new_from_frame_channel(HidFrameChannel::new(device));
        /*
            let m = Tlv::new_with_raw_val(0xDF46, vec![0x00, 0x00]).unwrap();

            h.request_get(m).unwrap();

            let rs = h
                .response(|x| true, std::time::Duration::from_millis(500))
                .unwrap();
        */
        //let r = undefine_nfc_reader::tlv_handler::get_serial(&h);
    println!("----------------------------");
        let h = Arc::new(h);

        for i in 0..2 {
            let s_h = h.clone();
            thread::spawn(move || {
                let m = Tlv::new_with_raw_val(0xDF46, vec![0x00, 0x00]).unwrap();

                //s_h.request_get(m).unwrap();

                let rs = s_h
                    .response(|x| true, std::time::Duration::from_millis(50000))
                    .unwrap();
            });
        }

        //drop(tlv_queue);
        thread::sleep(std::time::Duration::from_millis(5000));
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

        println!("{}", TlvDecorator::new(&t3));*/*/
}
