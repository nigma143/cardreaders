use hidapi::*;
use std::io::{Error, ErrorKind};
use std::io::{Read, Write};

mod error;
pub mod frame_channel;
pub mod message_channel;
mod number;

use error::ByteChannelError;

pub fn Do() {
    let hd_api = HidApi::new().unwrap();

    //let device = hd_api.open(0x1089, 0x0001).unwrap();

    for device in hd_api.device_list() {
        println!("{:?}", device.path());
    }

    let list: Vec<&DeviceInfo> = hd_api.device_list().collect();
    println!("{:?}", list.len());
}

enum Message<'a> {
    Ack,
    Nack { code: u16 },
    Set { payload: &'a [u8] },
    Do { payload: &'a [u8] },
}

struct FramelessHidDevice<'a> {
    hid_device: &'a HidDevice,
    read_timeout: i32,
    write_timeout: i32,
}
/*
impl FramelessHidDevice<'_> {
    fn new(hid_device: &HidDevice, read_timeout: i32, write_timeout: i32) -> FramelessHidDevice {
        FramelessHidDevice {
            hid_device,
            read_timeout,
            write_timeout,
        }
    }

    fn read(&mut self, read_timeout: i32) -> Result<[u8], HidError> {
        let mut result: Vec<u8> = Vec::new();

        let mut buf_f_size: [u8; 2] = Default::default();

        self.hid_device
            .read_timeout(&mut buf_f_size, read_timeout)?;

        let message_len = to_u16_big_endian(buf_f_size, 0).ok_or(HidError::HidApiError {
            message: "incorrect message length",
        })?;

        let readed_total = 0;
        while readed_total < message_len {
            let mut buf: [u8; 64] = Default::default();
            let readed = self.hid_device.read_timeout(&mut buf, read_timeout)?;
        }

        loop {
            let mut buf: [u8; 64] = Default::default();
            let readed = self.hid_device.read_timeout(&mut buf, read_timeout)?;
        }
    }
}

fn to_u16_big_endian(array: &[u8], start_index: usize) -> Option<u16> {
    let first_part = array.get(start_index)?;
    let second_part = array.get(start_index + 1)?;
    Ok((first_part << 8) + second_part)
}*/
