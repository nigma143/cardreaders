use crate::error::{ByteChannelError, MessageChannelError};

use byteorder::{BigEndian, ByteOrder};

pub trait FrameChannel {
    fn write(&self, frame: &[u8]) -> Result<(), ByteChannelError>;
    fn read(&self) -> Result<Vec<u8>, ByteChannelError>;
}

#[derive(Debug, Clone)]
pub enum Message {
    Ask,
    Nack(u16),
    Do(Vec<u8>),
    Get(Vec<u8>),
    Set(Vec<u8>),
}

pub struct MessageChannel<T>
where
    T: FrameChannel,
{
    channel: T,
}

impl<T> MessageChannel<T>
where
    T: FrameChannel,
{
    pub fn new(channel: T) -> Self {
        Self { channel }
    }

    pub fn write(&self, message: &Message) -> Result<(), MessageChannelError> {
        let payload = match message {
            Message::Do(payload) => payload,
            Message::Get(payload) => payload,
            Message::Set(payload) => payload,
            _ => Err(MessageChannelError::InvalidRequestMessageType())?,
        };
        let raw_message_size = 1 + //STX
        1 + //Unit
        1 + //Opcode
        payload.len() + //DATA
        1 + //LRC
        1; //ETX

        let mut raw_message = Vec::new();
        raw_message.push(0x02);
        raw_message.append(&mut Self::calculate_length_field(raw_message_size));
        raw_message.push(0x00);

        match message {
            Message::Do(_) => raw_message.push(0x3E),
            Message::Get(_) => raw_message.push(0x3D),
            Message::Set(_) => raw_message.push(0x3C),
            _ => Err(MessageChannelError::InvalidResponseMessageType())?,
        };

        raw_message.extend(payload.iter());

        raw_message.push(Self::calculate_lrc(&raw_message));
        raw_message.push(0x03);

        self.channel.write(&raw_message)?;

        Ok(())
    }

    pub fn read(&self) -> Result<Message, MessageChannelError> {
        let mut buf: Vec<u8> = Vec::new();

        while buf.len() < 6 {
            //STX + UNIT + OPCODE + LEN(1-3)
            let mut readed = self.channel.read()?;
            if readed.len() == 0 {
                return Err(MessageChannelError::Other(format!(
                    "read head block size is 0"
                )));
            }

            buf.append(&mut readed);
        }

        let stx = buf[0];

        if stx != 0x02 {
            return Err(MessageChannelError::Other(format!("expected STX")));
        }

        let (m_len, offset) = Self::get_message_length(&buf, 1)
            .ok_or(MessageChannelError::Other(format!("incorrect LEN")))?;

        while buf.len() < m_len as usize {
            let mut readed = self.channel.read()?;
            if readed.len() == 0 {
                return Err(MessageChannelError::Other(format!(
                    "read body block size is 0"
                )));
            }

            buf.append(&mut readed);
        }

        let opcode = buf[offset + 1];
        let payload_index = offset + 2;
        let lrc_index = buf.len() - 2;
        let lrc = buf[lrc_index];
        let ext = buf[lrc_index + 1];

        if ext != 0x03 {
            return Err(MessageChannelError::Other(format!("expected ETX")));
        }

        if lrc != Self::calculate_lrc(&buf[0..lrc_index]) {
            return Err(MessageChannelError::Other(format!("incorrect LRC")));
        }

        let payload = &buf[payload_index..lrc_index];

        match opcode {
            0x15 => Ok(Message::Nack(BigEndian::read_u16(&payload))),
            0x3E => Ok(Message::Do(payload.to_vec())),
            0x3D => Ok(match payload {
                [0x00, 0x00] => Message::Ask,
                _ => Message::Get(payload.to_vec()),
            }),
            0x3C => Ok(Message::Set(payload.to_vec())),
            _ => Err(MessageChannelError::Other(format!("inccorect OPCODE"))),
        }
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
}
