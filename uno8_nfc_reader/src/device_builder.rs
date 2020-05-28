use crate::device;
use crate::message_channel;

use std::time::Duration;

use hidapi::{HidApi, HidDevice, HidError};

use device::Uno8NfcDevice;
use message_channel::MessageChannel;

pub struct Uno8NfcDeviceBuilder<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    device: Uno8NfcDevice<TMessageChannel>,
}

impl Uno8NfcDeviceBuilder<HidDevice> {
    pub fn use_hid(vid: u16, pid: u16) -> Result<Uno8NfcDeviceBuilder<HidDevice>, HidError> {
        Ok(Self {
            device: Uno8NfcDevice::new(HidApi::new()?.open(vid, pid)?),
        })
    }
}

impl<TMessageChannel> Uno8NfcDeviceBuilder<TMessageChannel>
where
    TMessageChannel: MessageChannel,
{
    pub fn set_ack_timeout(mut self, timeout: Duration) -> Self {
        self.device.set_ack_timeout(timeout);
        self
    }

    pub fn finish(self) -> Uno8NfcDevice<TMessageChannel> {
        self.device
    }
}
