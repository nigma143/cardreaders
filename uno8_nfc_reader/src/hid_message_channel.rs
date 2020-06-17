use crate::error;
use crate::message_channel;

use byteorder::{BigEndian, ByteOrder};
use hidapi::{HidDevice, HidError};

use error::*;

use card_less_reader::tlv_parser::{Tlv, TlvError};
use message_channel::{MessageChannel, ReadMessage, WriteMessage};

impl MessageChannel for HidDevice {
    fn write(&self, message: &WriteMessage) -> Result<(), WriteMessageError> {
        let (op_code, payload) = match message {
            WriteMessage::Do(tlv) => (0x3E_u8, tlv.to_vec()),
            WriteMessage::Get(tlv) => (0x3D_u8, tlv.to_vec()),
            WriteMessage::Set(tlv) => (0x3C_u8, tlv.to_vec()),
        };
        let raw_message_size = 1 + //STX
        1 + //Unit
        1 + //Opcode
        payload.len() + //DATA
        1 + //LRC
        1; //ETX

        let mut raw_message = Vec::new();
        raw_message.push(0x02);
        raw_message.append(&mut calculate_length_field(raw_message_size));
        raw_message.push(0x00);
        raw_message.push(op_code);
        raw_message.extend(payload.iter());

        raw_message.push(calculate_lrc(&raw_message));
        raw_message.push(0x03);

        self.set_blocking_mode(true)?;
        write_frame_less(self, &raw_message)?;
        self.set_blocking_mode(false)?;

        Ok(())
    }

    fn try_read(&self) -> Result<ReadMessage, TryReadMessageError> {
        self.set_blocking_mode(false)?;

        let mut buf: Vec<u8> = vec![];

        let mut read = read_frame_less(self)?;

        if read.len() == 0 {
            return Err(TryReadMessageError::Empty);
        }

        if read[0] != 0x02 {
            return Err(TryReadMessageError::Other(format!("expected STX")));
        }
        buf.append(&mut read);

        while buf.len() < 5 {
            //UNIT + OPCODE + LEN(1-3)
            let mut read = read_frame_less(self)?;
            if read.len() > 0 {
                buf.append(&mut read);
            }
        }

        let (m_len, offset) = get_message_length(&buf, 1)
            .ok_or(ReadMessageError::Other(format!("incorrect field LEN")))?;

        while buf.len() < m_len as usize {
            let mut read = read_frame_less(self)?;
            if read.len() > 0 {
                buf.append(&mut read);
            }
        }

        let opcode = buf[offset + 1];
        let payload_index = offset + 2;
        let lrc_index = buf.len() - 2;
        let lrc = buf[lrc_index];
        let ext = buf[lrc_index + 1];

        if ext != 0x03 {
            return Err(TryReadMessageError::Other(format!("expected ETX")));
        }

        if lrc != calculate_lrc(&buf[0..lrc_index]) {
            return Err(TryReadMessageError::Other(format!("incorrect LRC")));
        }

        let payload = &buf[payload_index..lrc_index];

        self.set_blocking_mode(true)?;

        match opcode {
            0x15 => Ok(ReadMessage::Nack(payload[0])),
            0x3E => Ok(match payload {
                [0x00, 0x00] => ReadMessage::Ask,
                _ => ReadMessage::Do(Tlv::from_vec(payload)?),
            }),
            0x3D => Ok(match payload {
                [0x00, 0x00] => ReadMessage::Ask,
                _ => ReadMessage::Get(Tlv::from_vec(payload)?),
            }),
            0x3C => Ok(match payload {
                [0x00, 0x00] => ReadMessage::Ask,
                _ => ReadMessage::Set(Tlv::from_vec(payload)?),
            }),
            _ => Err(TryReadMessageError::Other(format!("inccorect OPCODE"))),
        }
    }
}

fn write_frame_less(device: &HidDevice, frame: &[u8]) -> Result<(), WriteMessageError> {
    let chunks: Vec<&[u8]> = frame.chunks(63).collect();
    for i in 0..chunks.len() {
        let mut frame = Vec::new();
        frame.push(0x00);

        if i < chunks.len() {
            frame.push(chunks[i].len() as u8);
        } else {
            frame.push(0xFF);
        }

        frame.extend_from_slice(chunks[i]);

        while frame.len() < 65 {
            frame.push(0x00);
        }

        log::info!("write: {:02X?}", frame);

        let w_count = device.write(&frame)?;
        if w_count != frame.len() {
            return Err(WriteMessageError::Other(format!(
                "incorrect write byte count"
            )));
        }
    }

    Ok(())
}

fn read_frame_less(device: &HidDevice) -> Result<Vec<u8>, ReadMessageError> {
    let mut buf: [u8; 64] = [0; 64];

    let count = device.read(&mut buf)?;
    if count == 0 {
        return Ok(vec![]);
    }

    log::info!("read: {:02X?}", buf.to_vec());

    if count != buf.len() {
        return Err(ReadMessageError::Other(format!(
            "head read size is incorrect"
        )));
    }

    let m_len = buf[0] as usize;

    Ok(buf[1..(m_len + 1)].to_vec())
}

fn calculate_length_field(byte_size: usize) -> Vec<u8> {
    if byte_size + 1 <= 0x7F {
        vec![(byte_size + 1) as u8]
    } else if byte_size + 2 <= 0xFF {
        vec![0x81, (byte_size + 2) as u8]
    } else if byte_size + 3 <= 0xFFFF {
        let mut vec: Vec<u8> = vec![0x82];
        BigEndian::write_u16(&mut vec, byte_size as u16);
        vec
    } else {
        panic!("incorrect message size");
    }
}

fn calculate_lrc(buf: &[u8]) -> u8 {
    let mut lrc: u8 = 0;
    for b in buf {
        lrc ^= b;
    }
    return lrc;
}

fn get_message_length(buf: &[u8], offset: usize) -> Option<(u16, usize)> {
    if buf[offset] == 0x81 {
        Some((buf[offset + 1] as u16, offset + 2))
    } else if buf[offset] == 0x82 {
        Some((
            BigEndian::read_u16(&buf[(offset + 1)..=(offset + 3)]),
            offset + 3,
        ))
    } else {
        Some((buf[offset] as u16, offset + 1))
    }
}

impl From<HidError> for WriteMessageError {
    fn from(error: HidError) -> Self {
        WriteMessageError::Other(format!("{}", error))
    }
}

impl From<HidError> for ReadMessageError {
    fn from(error: HidError) -> Self {
        ReadMessageError::Other(format!("{}", error))
    }
}

impl From<TlvError> for ReadMessageError {
    fn from(error: TlvError) -> Self {
        ReadMessageError::Other(format!("{}", error))
    }
}

impl From<ReadMessageError> for TryReadMessageError {
    fn from(error: ReadMessageError) -> Self {
        TryReadMessageError::Other(format!("{}", error))
    }
}

impl From<HidError> for TryReadMessageError {
    fn from(error: HidError) -> Self {
        TryReadMessageError::Other(format!("{}", error))
    }
}

impl From<TlvError> for TryReadMessageError {
    fn from(error: TlvError) -> Self {
        TryReadMessageError::Other(format!("{}", error))
    }
}
