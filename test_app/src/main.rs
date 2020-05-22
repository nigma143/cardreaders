use hidapi::*;
use undefine_nfc_reader::frame_channel::HidFrameChannel;
use undefine_nfc_reader::message_channel::{Message, MessageChannel};
use tlv_parser::tlv::Tlv;
use std::fmt;

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
        _ => panic!(":dsfdsfdsf")
    };


    let tlv = Tlv::from_vec(&payload).unwrap();

    println!("{}", TlvExtended { tlv: tlv });
}

struct TlvExtended {
    tlv: Tlv
}

impl TlvExtended {
    fn fmt_ext(tlv: &Tlv, ident: &mut String, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}- {:02X}: ", &ident, tlv.tag())?;
        match tlv.val() {            
            tlv_parser::tlv::Value::Val(val) => {
                write!(f, "{:02X?}", val)
            }
            tlv_parser::tlv::Value::TlvList(childs) => {
                writeln!(f)?;
                ident.push_str("  ");
                for child in childs {
                    Self::fmt_ext(child, ident, f)?;
                }
                Ok(())
            }
            tlv_parser::tlv::Value::Nothing => {
                write!(f, "")
            }
        }
    }
}

impl fmt::Display for TlvExtended {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Self::fmt_ext(&self.tlv, &mut "".to_owned(), f)
    }
}
