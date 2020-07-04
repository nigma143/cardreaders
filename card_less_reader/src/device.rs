use crate::error;
use crate::tlv_parser;

use error::*;
use std::{io::{Read, Write, BufReader}, sync::{atomic::AtomicBool, Arc}};
use tlv_parser::Tlv;

type StorageResult<T> = Result<T, StorageError>;

pub trait CardLessDevice {
    fn get_sn(&self) -> Result<String, DeviceError>;

    fn poll_emv(
        &mut self,
        purchase: Option<PollEmvPurchase>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<PollEmvResult, DeviceError>;

    fn ext_display(&mut self) -> Option<&dyn ExtDisplay>;

    fn storage(&mut self) -> Option<&dyn Storage>;
}

pub trait ExtDisplay {
    fn get_display_mode(&self) -> Result<ExtDisplayMode, DeviceError>;

    fn set_display_mode(&self, mode: &ExtDisplayMode) -> Result<(), DeviceError>;
}

#[derive(Debug)]
pub struct PollEmvPurchase {
    pub p_type: u8,
    pub currency_code: u16,
    pub amount: u64,
}

impl PollEmvPurchase {
    pub fn new(p_type: u8, currency_code: u16, amount: u64) -> Self {
        Self {
            p_type,
            currency_code,
            amount,
        }
    }
}

pub enum PollEmvResult {
    Canceled,
    Success(Tlv),
}

#[derive(Copy, Clone)]
pub enum ExtDisplayMode {
    NoDisplay,
    Simple,
    Full,
}

pub trait Storage {
    fn dir_exist(&self, path: String) -> StorageResult<()>;

    fn get_dir_list(&self, path: String) -> StorageResult<Vec<String>>;

    fn create_dir(&self, path: String) -> StorageResult<()>;

    fn delete_dir(&self, path: String) -> StorageResult<()>;

    fn file_exist(&self, file_path: String) -> StorageResult<()>;

    fn get_file_list(&self, path: String) -> StorageResult<Vec<String>>;

    fn delete_file(&self, file_path: String) -> StorageResult<()>;

    fn open_read_file(&self, file_path: String) -> StorageResult<&dyn Read>;

    fn open_write_file(&self, file_path: String) -> StorageResult<&dyn Write>;
/*
    fn read_file(&self, file_path: String) -> StorageResult<Vec<u8>> {
        let mut buf_reader = BufReader::new(
            self.open_read_file(file_path));

       buf_reader.     
    }*/
}