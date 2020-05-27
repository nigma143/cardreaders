use crate::error;
use crate::message_channel;
use crate::tlv_channel;

use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use tlv_parser::tlv::{Tlv, Value};

use error::*;
use message_channel::{FrameChannel, MessageChannel};
use tlv_channel::{ReadTlv, TlvChannel, WriteTlv};
/*
pub fn get_serial(common_handler: &TlvHandler) -> Tlv {
    let handler = common_handler.create_handler_scope(Box::new(|x| contains(x, &[0xDF4D])));

    handler.request_get(Tlv::new(0xDF4D, Value::Nothing).unwrap()).unwrap();
    handler.response(Duration::from_millis(1000)).unwrap()
}

fn contains(tlv: &Tlv, tags: &[usize]) -> bool {
    match tlv.val() {
        tlv_parser::tlv::Value::TlvList(childs) => {
            match childs.iter().find(|x| tags.contains(&x.tag())) {
                Some(_) => true,
                None => false
            }
        }
        tlv_parser::tlv::Value::Val(_) => false,
        tlv_parser::tlv::Value::Nothing => false
    }
}

pub struct TlvHandlerScope<'a> {
    handler: &'a TlvHandler,
    match_fn: Box<dyn Fn(&Tlv) -> bool + Send>,
}

impl<'a> TlvHandlerScope<'a> {
    pub fn new(handler: &'a TlvHandler, match_fn: Box<dyn Fn(&Tlv) -> bool + Send>) -> Self {
        Self {
            handler: handler,
            match_fn: match_fn,
        }
    }

    pub fn request_do(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.handler.request_do(tlv)
    }

    pub fn request_get(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.handler.request_get(tlv)
    }

    pub fn request_set(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.handler.request_set(tlv)
    }

    pub fn response(&self, timeout: Duration) -> Result<Tlv, TlvQueueError> {
        self.handler.response(self.match_fn, timeout)
    }
}
*/
pub struct TlvHandler<TFrameChannel>
where
    TFrameChannel: FrameChannel + Send + 'static,
{
    tlv_channel: Arc<TlvChannel<TFrameChannel>>,
    sender: Mutex<Sender<ReadRegistration>>,
}

impl<TFrameChannel> TlvHandler<TFrameChannel>
where
    TFrameChannel: FrameChannel + Send + Sync + 'static,
{
    pub fn new_from_frame_channel(frame_channel: TFrameChannel) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        TlvHandler::new(TlvChannel::new(MessageChannel::new(frame_channel)))
    }

    pub fn new(tlv_channel: TlvChannel<TFrameChannel>) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        let shared_tlv_ch = Arc::new(tlv_channel);

        let read_handler_tlv_ch = shared_tlv_ch.clone();
        let (sender, receiver) = channel();

        thread::spawn(|| TlvHandler::run_read_handler(read_handler_tlv_ch, receiver));

        Self {
            tlv_channel: shared_tlv_ch.clone(),
            sender: Mutex::new(sender),
        }
    }

    pub fn request_do(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.request(WriteTlv::Do(tlv))
    }

    pub fn request_get(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.request(WriteTlv::Get(tlv))
    }

    pub fn request_set(&self, tlv: Tlv) -> Result<(), TlvQueueError> {
        self.request(WriteTlv::Set(tlv))
    }

    fn request(&self, data: WriteTlv) -> Result<(), TlvQueueError> {        
        self.tlv_channel.write(&data)?;
        let (sender, receiver) = channel();
        self.sender
            .lock()
            .unwrap()
            .send(ReadRegistration::Ack(ReadAckRegistration::new(sender)))
            .map_err(|_| TlvQueueError::Disconnected)?;
        let result = receiver.recv_timeout(Duration::from_millis(300))?;
        match result {
            Ok(o) => Ok(o),
            Err(e) => Err(TlvQueueError::PutError(e)),
        }
    }

    pub fn response(
        &self,
        match_fn: impl Fn(&Tlv) -> bool + Send + 'static,
        timeout: Duration,
    ) -> Result<Tlv, TlvQueueError> {
        let (sender, receiver) = channel();
        self.sender
            .lock()
            .unwrap()
            .send(ReadRegistration::Tlv(ReadTlvRegistration::new(
                sender, match_fn,
            )))
            .map_err(|_| TlvQueueError::Disconnected)?;
        Ok(receiver.recv_timeout(timeout)?)
    }
    /*
        pub fn create_handler_scope(&self, match_fn: Box<dyn Fn(&Tlv) -> bool + Send>) -> TlvHandlerScope {
            TlvHandlerScope::new(self, match_fn)
        }
    */
    fn run_read_handler(
        tlv_channel: Arc<TlvChannel<TFrameChannel>>,
        receiver: Receiver<ReadRegistration>,
    ) -> ()
    where
        TFrameChannel: FrameChannel,
    {
        let mut ack_awaiters = Vec::new();
        let mut awaiters = Vec::new();

        loop {
            match receiver.recv() {
                Ok(reg) => match reg {
                    ReadRegistration::Ack(r_reg) => ack_awaiters.push(r_reg),
                    ReadRegistration::Tlv(r_reg) => awaiters.push(r_reg),
                },
                Err(_) => {
                    log::trace!("sending channel disconnected");
                    break;
                }
            }

            match tlv_channel.read() {
                Ok(read) => match read {
                    ReadTlv::Ack => match ack_awaiters.first() {
                        Some(ack_awaiter) => {
                            match ack_awaiter.finish(Ok(())) {
                                Ok(_) => {}
                                Err(_) => {
                                    log::error!(
                                        "error on ack finish: receiving channel disconnected"
                                    );
                                    break;
                                }
                            }
                            ack_awaiters.remove(0);
                        }
                        None => {
                            log::error!("ack read is not expected");
                            break;
                        }
                    },
                    ReadTlv::Nack(code) => match ack_awaiters.first() {
                        Some(ack_awaiter) => {
                            match ack_awaiter.finish(Err(code)) {
                                Ok(_) => {}
                                Err(_) => {
                                    log::error!(
                                        "error on nack finish: receiving channel disconnected"
                                    );
                                    break;
                                }
                            }
                            ack_awaiters.remove(0);
                        }
                        None => {
                            log::error!("ack read is not expected");
                            break;
                        }
                    },
                    ReadTlv::Tlv(tlv) => {
                        match awaiters
                            .iter()
                            .position(|x| x.is_match(&tlv))
                            .map(|x| awaiters.remove(x))
                        {
                            Some(awaiter) => match awaiter.finish(tlv) {
                                Ok(_) => {}
                                Err(_) => {
                                    log::error!(
                                        "error on tlv finish: receiving channel disconnected"
                                    );
                                    break;
                                }
                            },
                            None => {}
                        }
                    }
                },
                Err(e) => {
                    log::trace!("error on read from tlv channel: {:?}", e);
                    break;
                }
            }
        }
    }
}

enum ReadRegistration {
    Ack(ReadAckRegistration),
    Tlv(ReadTlvRegistration),
}

struct ReadAckRegistration {
    sender: Sender<Result<(), u8>>,
}

impl ReadAckRegistration {
    fn new(sender: Sender<Result<(), u8>>) -> Self {
        Self { sender }
    }
    fn finish(&self, result: Result<(), u8>) -> Result<(), SendError<Result<(), u8>>> {
        self.sender.send(result)
    }
}

struct ReadTlvRegistration {
    sender: Sender<Tlv>,
    match_fn: Box<dyn Fn(&Tlv) -> bool + Send>,
}

impl ReadTlvRegistration {
    fn new(sender: Sender<Tlv>, match_fn: impl Fn(&Tlv) -> bool + Send + 'static) -> Self {
        Self {
            sender: sender,
            match_fn: Box::new(match_fn),
        }
    }
    fn is_match(&self, data: &Tlv) -> bool {
        (self.match_fn)(data)
    }
    fn finish(&self, data: Tlv) -> Result<(), SendError<Tlv>> {
        self.sender.send(data)
    }
}
