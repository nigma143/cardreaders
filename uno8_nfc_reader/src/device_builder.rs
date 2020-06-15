use crate::device;
use crate::message_channel;

use std::time::Duration;

use hidapi::{HidApi, HidDevice, HidError};

use card_less_reader::device::CardLessDevice;
use device::Uno8NfcDevice;
use message_channel::MessageChannel;

pub struct Uno8NfcDeviceBuilder {
    device: Uno8NfcDevice,
}

impl Uno8NfcDeviceBuilder {
    pub fn use_hid(vid: u16, pid: u16) -> Result<Uno8NfcDeviceBuilder, HidError> {
        Ok(Self {
            device: Uno8NfcDevice::new(HidApi::new()?.open(vid, pid)?),
        })
    }
}

impl Uno8NfcDeviceBuilder {
    pub fn set_ack_timeout(mut self, timeout: Duration) -> Self {
        self.device.set_ack_timeout(timeout);
        self
    }

    pub fn set_read_timeout(mut self, timeout: Duration) -> Self {
        self.device.set_read_timeout(timeout);
        self
    }

    pub fn set_external_display(mut self, f: impl Fn(&String) + Send + 'static) -> Self {
        self.device.set_external_display(f);
        self
    }

    pub fn set_internal_log(mut self, f: impl Fn(&String) + Send + 'static) -> Self {
        self.device.set_internal_log(f);
        self
    }

    pub fn set_card_removal(mut self, f: impl Fn() + Send + 'static) -> Self {
        self.device.set_card_removal(f);
        self
    }

    pub fn finish(self) -> Uno8NfcDevice {
        self.device
    }
}
