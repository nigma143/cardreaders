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

pub struct TlvQueue {
    read_thread: thread::JoinHandle<()>,
    sender: Sender<Registration>,
}

impl TlvQueue {
    pub fn new<TFrameChannel>(channel: TlvChannel<TFrameChannel>) -> Self
    where
        TFrameChannel: FrameChannel + Send + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            read_thread: thread::spawn(|| tlv_queue_handler(channel, receiver)),
            sender: sender,
        }
    }

    pub fn put(&self, data: WriteTlv) -> Result<(), TlvQueueError> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.sender
            .send(Registration::Write(WriteRegistration::new(sender, data)))
            .unwrap();

        let result = receiver.recv_timeout(Duration::from_millis(300))?;
        match result {
            Ok(o) => Ok(o),
            Err(e) => Err(TlvQueueError::PutError(e)),
        }
    }

    pub fn get<F>(&self, match_fn: F, timeout: Duration) -> Result<Tlv, RecvTimeoutError>
    where
        F: Fn(&Tlv) -> bool + Send + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.sender
            .send(Registration::Read(ReadRegistration::new(sender, match_fn)))
            .unwrap();

        receiver.recv_timeout(timeout)
    }
}

fn tlv_queue_handler<TFrameChannel>(
    channel: TlvChannel<TFrameChannel>,
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
                Registration::Write(w_reg) => match channel.write(&w_reg.get_write_data()) {
                    Ok(_) => {
                        ack_awaiters.push(w_reg);
                    }
                    Err(e) => {
                        log::error!("error on write to channel: {:?}", e);
                        continue;
                    }
                },
                Registration::Read(r_reg) => {
                    awaiters.push(r_reg);
                }
            },
            Err(_) => {
                log::info!("reciever disconected");
                break;
            }
        }

        match channel.read() {
            Ok(read) => match read {
                ReadTlv::Ack => match ack_awaiters.first() {
                    Some(ack_awaiter) => {
                        match ack_awaiter.finish(Ok(())) {
                            Ok(_) => {}
                            Err(_) => {
                                log::error!("error on ack finish");
                                continue;
                            }
                        }
                        ack_awaiters.remove(0);
                    }
                    None => {
                        log::error!("ack read is not expected");
                        continue;
                    }
                },
                ReadTlv::Nack(code) => match ack_awaiters.first() {
                    Some(ack_awaiter) => {
                        match ack_awaiter.finish(Err(code)) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("error on nack finish: {:?}", e);
                                continue;
                            }
                        }
                        ack_awaiters.remove(0);
                    }
                    None => {
                        log::error!("ack read is not expected");
                        continue;
                    }
                },
                ReadTlv::Tlv(tlv) => {
                    match awaiters
                        .iter()
                        .position(|x| x.is_match(&tlv))
                        .map(|x| awaiters.remove(x))
                    {
                        Some(awaiter) => {
                            match awaiter.finish(tlv) {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!("error on tlv finish: {}", e);
                                    continue;
                                }
                            }
                        }
                        None => {}
                    }
                }
            },
            Err(e) => {
                log::info!("error on read from channel: {:?}", e);
                continue;
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
    match_fn: Box<Fn(&Tlv) -> bool + Send + 'static>,
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
