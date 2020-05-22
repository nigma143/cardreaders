use cancellation::{CancellationToken, CancellationTokenSource, OperationCanceled};

use crate::error::{ByteChannelError, MessageChannelError};
use crate::number;

pub trait FrameChannel {
    fn write(&self, frame: &[u8], ct: &CancellationToken) -> Result<(), ByteChannelError>;
    fn read(&self, ct: &CancellationToken) -> Result<Vec<u8>, ByteChannelError>;
}

pub enum Message {
    Ask,
    Nack { code: u16 },
    Do { payload: Vec<u8> },
    Get { payload: Vec<u8> },
    Set { payload: Vec<u8> },
}

pub struct MessageChannel<T>
where
    T: FrameChannel,
{
    channel: T,
}

#[allow(dead_code)]
impl<T> MessageChannel<T>
where
    T: FrameChannel,
{
    pub fn new(channel: T) -> Self {
        MessageChannel { channel }
    }

    pub fn write(&self, m: &mut Message) -> Result<(), MessageChannelError> {
        self.write_ct(m, &CancellationTokenSource::new())
    }

    pub fn write_ct(
        &self,
        m: &mut Message,
        ct: &CancellationToken,
    ) -> Result<(), MessageChannelError> {
        self.write_impl(m, |x| self.channel.write(x, ct))?;
        Ok(())
    }

    pub fn read(&self) -> Result<Message, MessageChannelError> {
        self.read_ct(&CancellationTokenSource::new())
    }

    pub fn read_ct(&self, ct: &CancellationToken) -> Result<Message, MessageChannelError> {
        self.read_impl(|| self.channel.read(ct))
    }

    fn write_impl<F>(&self, message: &mut Message, write: F) -> Result<(), MessageChannelError>
    where
        F: Fn(&[u8]) -> Result<(), ByteChannelError>,
    {
        let payload_size = match message {
            Message::Do { payload } => payload.len(),
            Message::Get { payload } => payload.len(),
            Message::Set { payload } => payload.len(),
            _ => panic!("invalid message type"),
        };
        let raw_message_size = 1 + //STX
        1 + //Unit
        1 + //Opcode
        payload_size + //DATA
        1 + //LRC
        1; //ETX

        let mut raw_message = Vec::new();
        raw_message.push(0x02);
        raw_message.append(&mut number::calculate_length_field(raw_message_size));
        raw_message.push(0x00);

        match message {
            Message::Do { ref mut payload } => {
                raw_message.push(0x3E);
                raw_message.append(payload);
            }
            Message::Get { ref mut payload } => {
                raw_message.push(0x3D);
                raw_message.append(payload);
            }
            Message::Set { ref mut payload } => {
                raw_message.push(0x3C);
                raw_message.append(payload);
            }
            _ => panic!("invalid message type"),
        };

        raw_message.push(number::calculate_lrc(&raw_message));
        raw_message.push(0x03);

        write(&raw_message)?;

        Ok(())
    }

    fn read_impl<F>(&self, read: F) -> Result<Message, MessageChannelError>
    where
        F: Fn() -> Result<Vec<u8>, ByteChannelError>,
    {
        let mut buf: Vec<u8> = Vec::new();

        while buf.len() < 6 {
            //STX + UNIT + OPCODE + LEN(1-3)
            let mut readed = read()?;
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

        let (m_len, offset) = number::get_message_length(&buf, 1)
            .ok_or(MessageChannelError::Other(format!("incorrect LEN")))?;

        while buf.len() < m_len as usize {
            let mut readed = read()?;
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

        if lrc != number::calculate_lrc(&buf[0..lrc_index]) {
            return Err(MessageChannelError::Other(format!("incorrect LRC")));
        }

        let payload = &buf[payload_index..lrc_index];

        match opcode {
            0x15 => Ok(Message::Nack {
                code: number::to_u16_big_endian(&payload).ok_or(
                    MessageChannelError::Other(format!("incorrect NACK error code block")),
                )?,
            }),
            0x3E => Ok(Message::Do {
                payload: payload.to_vec(),
            }),
            0x3D => Ok(match payload {
                [0x00, 0x00] => Message::Ask,
                _ => Message::Get {
                    payload: payload.to_vec(),
                },
            }),
            0x3C => Ok(Message::Set {
                payload: payload.to_vec(),
            }),
            _ => Err(MessageChannelError::Other(format!("inccorect OPCODE"))),
        }
    }
}
