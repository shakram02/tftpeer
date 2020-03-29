use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::Path;

use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};
use crate::tftp::shared::STRIDE_SIZE;

#[derive(Debug, Eq, PartialEq)]
pub enum DataChannelMode {
    Tx,
    Rx,
}

pub struct TFTPDataChannel {
    fd: File,
    bytes: usize,
    blk: isize,
    done: bool,
}

impl TFTPDataChannel {
    /// Makes a new TFTPDataChannel with is backed by a File that's open
    /// in either read or write modes. If opening the File fails, an Error
    /// is returned.
    ///
    /// * `file_name` - Specified file name to read data from / write data to.
    /// * `channel_mode` - Tells whether this channel will be receiving or sending data.
    pub fn new(file_name: &str, channel_mode: DataChannelMode) -> Result<Self, Error> {
        let (initial_blk, fd) = match channel_mode {
            DataChannelMode::Tx => (-1, File::open(Path::new(file_name))),
            DataChannelMode::Rx => (0, File::create(Path::new(file_name))),
        };

        if fd.is_err() {
            return Err(fd.unwrap_err());
        }

        let channel = TFTPDataChannel {
            fd: fd.unwrap(),
            bytes: 0,
            blk: initial_blk,
            done: false,
        };

        Ok(channel)
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn transfer_size(&self) -> usize {
        self.bytes
    }

    pub fn blk(&self) -> u16 {
        self.blk as u16
    }

    /// Receives a data packet and checks its block number,
    /// if the packets block number is invalid an ErrorPacket is
    /// returned, otherwise an AckPacket is returned
    ///
    /// * `dp` - Data packet received from the other end.
    pub fn receive_data(&mut self, dp: DataPacket) -> Result<AckPacket, ErrorPacket> {
        if (self.blk + 1) as u16 != dp.blk() {
            return Err(ErrorPacket::new(TFTPError::IllegalOperation));
        }

        self.blk = dp.blk() as isize;
        let data = &dp.data();
        self.bytes += data.len();
        self.fd.write_all(data).unwrap();

        // Final block, close file.
        if data.len() != STRIDE_SIZE {
            self.done = true
        }

        Ok(TFTPDataChannel::make_ack(self.blk as u16))
    }

    /// Makes an Acknowledgement packet
    ///
    /// * `blk` - Block number for the ack packet
    pub fn make_ack(blk: u16) -> AckPacket {
        AckPacket::new(blk)
    }

    pub fn receive_ack(&mut self, ap: AckPacket) -> Result<DataPacket, ErrorPacket> {
        if (self.blk + 1) as u16 != ap.blk() {
            return Err(ErrorPacket::new(TFTPError::IllegalOperation));
        }

        Ok(self.send_data())
    }

    /// Reads the next data packet to be sent,
    /// if this is the last packet, done will be
    /// set to true.
    fn send_data(&mut self) -> DataPacket {
        self.blk += 1;

        let mut buf = [0; STRIDE_SIZE];
        let bytes_read = self.fd.read(&mut buf).unwrap();
        self.bytes += bytes_read;

        if bytes_read < STRIDE_SIZE {
            self.done = true;
        }

        let data = Vec::from(&buf[0..bytes_read]);
        DataPacket::new(self.blk as u16, data)
    }
}
