use crate::error;
use crate::message_channel;

use std::sync::RwLock;

use hidapi::{HidDevice, HidError};

use error::*;
use message_channel::FrameChannel;

unsafe impl Sync for HidFrameChannel {}

pub struct HidFrameChannel {
    device: RwLock<HidDevice>,
}

impl HidFrameChannel {
    pub fn new(device: HidDevice) -> Self {
        HidFrameChannel {
            device: RwLock::new(device),
        }
    }        
}

impl FrameChannel for HidFrameChannel {
    fn write(&self, frame: &[u8]) -> Result<(), ByteChannelError> {
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

            let w_count = self.device.write().unwrap().write(&frame)?;
            if w_count != frame.len() {
                return Err(ByteChannelError::Other(format!(
                    "incorrect write byte count"
                )));
            }
        }

        Ok(())
    }

    fn read(&self) -> Result<Vec<u8>, ByteChannelError> {
        let mut buf: [u8; 64] = [0; 64];

        let r_count = self.device.read().unwrap().read(&mut buf)?;

        log::info!("read: {:02X?}", buf.to_vec());

        if r_count != buf.len() {
            return Err(ByteChannelError::Other(format!(
                "head read size is incorrect"
            )));
        }

        let m_len = buf[0] as usize;

        Ok(buf[1..(m_len + 1)].to_vec())
    }
}

impl From<HidError> for ByteChannelError {
    fn from(error: HidError) -> Self {
        ByteChannelError::Other(format!("{}", error))
    }
}
