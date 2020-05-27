pub mod command;
mod error;
pub mod frame_channel;
pub mod message_channel;
pub mod tlv;
pub mod tlv_channel;
pub mod tlv_handler;
/*
use tlv_parser::tlv::{Tlv, Value};

use message_channel::{FrameChannel};
use tlv_handler::{TlvHandler, TlvHandlerScope};

pub struct UndefineNfcDevice {
    handler: TlvHandler
}

impl UndefineNfcDevice {
    pub fn new_from_frame_channel<TFrameChannel>(frame_channel: TFrameChannel) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        Self {
            handler: TlvHandler::new_from_frame_channel(frame_channel)
        }
    }
/*
    fn create_handler_scope(&self, react_tags: &[usize]) -> TlvHandlerScope {
        self.handler.create_handler_scope(|tlv| UndefineNfcDevice::contains(tlv, react_tags))
    }*/
    fn contains(tlv: &Tlv, react_tags: &[usize]) -> bool {
        match tlv.val() {
            tlv_parser::tlv::Value::TlvList(childs) => {
                match childs.iter().find(|x| react_tags.contains(&x.tag())) {
                    Some(_) => true,
                    None => false
                }
            }
            tlv_parser::tlv::Value::Val(_) => false,
            tlv_parser::tlv::Value::Nothing => false
        }
    }
}*/
