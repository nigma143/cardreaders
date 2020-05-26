use crate::error;
use crate::message_channel;
use crate::tlv_channel;

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tlv_parser::tlv::Tlv;

use error::*;
use message_channel::{FrameChannel, MessageChannel};
use tlv_channel::{ReadTlv, TlvChannel, WriteTlv};

pub struct SmartTlvHandler<'a> {
    handler: &'a TlvHandler,
    match_fn: Arc<Box<Fn(&Tlv) -> bool + 'static>>,
}

impl<'a> SmartTlvHandler<'a> {
    pub fn new<F>(handler: &'a TlvHandler, match_fn: F) -> Self
    where
        F: Fn(&Tlv) -> bool + 'static,
    {
        Self {
            handler: handler,
            match_fn: Arc::new(Box::new(match_fn)),
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
/*
    pub fn response(&self, timeout: Duration) -> Result<Tlv, TlvQueueError> {
        self.handler.response(self.match_fn.clone()., timeout)
    }*/
}

pub struct TlvHandler {
    sender: Sender<Registration>,
}

impl TlvHandler {
    pub fn new_from_frame_channel<TFrameChannel>(frame_channel: TFrameChannel) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        TlvHandler::new(TlvChannel::new(MessageChannel::new(frame_channel)))
    }

    pub fn new<TFrameChannel>(tlv_channel: TlvChannel<TFrameChannel>) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        let (sender, receiver) = channel();

        thread::spawn(|| TlvHandler::run_tlv_handler(tlv_channel, receiver));

        Self { sender: sender }
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
        let (sender, receiver) = channel();
        self.sender
            .send(Registration::Write(WriteRegistration::new(sender, data)))
            .map_err(|_| TlvQueueError::Disconnected)?;
        let result = receiver.recv_timeout(Duration::from_millis(300))?;
        match result {
            Ok(o) => Ok(o),
            Err(e) => Err(TlvQueueError::PutError(e)),
        }
    }

    pub fn response<F>(&self, match_fn: F, timeout: Duration) -> Result<Tlv, TlvQueueError>
    where
        F: Fn(&Tlv) -> bool + Send + 'static,
    {
        let (sender, receiver) = channel();
        self.sender
            .send(Registration::Read(ReadRegistration::new(sender, match_fn)))
            .map_err(|_| TlvQueueError::Disconnected)?;
        Ok(receiver.recv_timeout(timeout)?)
    }

    fn run_tlv_handler<TFrameChannel>(
        tlv_channel: TlvChannel<TFrameChannel>,
        receiver: Receiver<Registration>,
    ) -> ()
    where
        TFrameChannel: FrameChannel,
    {
        let mut ack_awaiters = Vec::new();
        let mut awaiters = Vec::new();

        loop {
            match receiver.recv() {
                Ok(reg) => match reg {
                    Registration::Write(w_reg) => {
                        match tlv_channel.write(&w_reg.get_write_data()) {
                            Ok(_) => {
                                ack_awaiters.push(w_reg);
                            }
                            Err(e) => {
                                log::error!("error on write to tlv channel: {:?}", e);
                                break;
                            }
                        }
                    }
                    Registration::Read(r_reg) => {
                        awaiters.push(r_reg);
                    }
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

enum Registration {
    Write(WriteRegistration),
    Read(ReadRegistration),
}

struct WriteRegistration {
    sender: Sender<Result<(), u16>>,
    write_data: WriteTlv,
}

impl WriteRegistration {
    fn new(sender: Sender<Result<(), u16>>, data: WriteTlv) -> Self {
        Self {
            sender: sender,
            write_data: data,
        }
    }
    fn get_write_data(&self) -> &WriteTlv {
        &self.write_data
    }
    fn finish(&self, result: Result<(), u16>) -> Result<(), SendError<Result<(), u16>>> {
        self.sender.send(result)
    }
}

struct ReadRegistration {
    sender: Sender<Tlv>,
    match_fn: Box<dyn Fn(&Tlv) -> bool + Send + 'static>,
}

impl ReadRegistration {
    fn new<F>(sender: Sender<Tlv>, match_fn: F) -> Self
    where
        F: Fn(&Tlv) -> bool + Send + 'static,
    {
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
