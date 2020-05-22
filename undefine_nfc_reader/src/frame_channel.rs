use crate::error::*;
use crate::message_channel::FrameChannel;

use cancellation::{CancellationToken, OperationCanceled};
use hidapi::{HidDevice, HidError};

pub struct HidFrameChannel {
    device: HidDevice,
}

impl HidFrameChannel {
    pub fn new(device: HidDevice) -> Self {
        HidFrameChannel { device }
    }

    fn write_frame<F>(&self, frame: &[u8], write: F) -> Result<(), ByteChannelError>
    where
        F: Fn(&[u8]) -> Result<usize, ByteChannelError>,
    {
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

            println!("write: {:02X?}", frame);

            let w_count = write(&frame)?;
            if w_count != frame.len() {
                return Err(ByteChannelError::Other(format!(
                    "incorrect write byte count"
                )));
            }
        }

        Ok(())
    }

    fn read_frame<F>(&self, ct: &CancellationToken, read: F) -> Result<Vec<u8>, ByteChannelError>
    where
        F: Fn(&mut [u8]) -> Result<usize, ByteChannelError>,
    {
        let mut buf: [u8; 64] = [0; 64];

        loop {
            ct.result()?;

            let r_count = read(&mut buf)?;
            if r_count == 0 {
                continue;
            }

            println!("read: {:02X?}", buf.to_vec());

            if r_count != buf.len() {
                return Err(ByteChannelError::Other(format!(
                    "head read size is incorrect"
                )));
            }
            break;
        }

        let m_len = buf[0] as usize;

        Ok(buf[1..(m_len + 1)].to_vec())
    }
}

impl FrameChannel for HidFrameChannel {
    fn write(&self, frame: &[u8], ct: &CancellationToken) -> Result<(), ByteChannelError> {
        Ok(self.write_frame(frame, |x| Ok(self.device.write(x)?))?)
    }

    fn read(&self, ct: &CancellationToken) -> Result<Vec<u8>, ByteChannelError> {
        Ok(self.read_frame(ct, |x| Ok(self.device.read(x)?))?)
    }
}

impl From<HidError> for ByteChannelError {
    fn from(error: HidError) -> Self {
        ByteChannelError::Other(format!("{}", error))
    }
}

impl From<OperationCanceled> for ByteChannelError {
    fn from(error: OperationCanceled) -> Self {
        ByteChannelError::OperationCanceled()
    }
}
