use std::fs::File;
use std::io::{Read, Write};
use std::mem;
use std::path::Path;

use crate::tftp::shared::{Serializable, STRIDE_SIZE};
use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};

#[derive(Debug, Eq, PartialEq)]
pub enum DataChannelMode {
    Tx,
    Rx,
}

#[derive(Debug, Eq, PartialEq)]
enum DataChannelState {
    WaitData,
    SendAck,
    SendLastAck,
    SendData,
    WaitAck,
    WaitLastAck,
    Error,
    Done,
}

pub struct DataChannel {
    fd: File,
    bytes: usize,
    blk: isize,
    error: Option<String>,
    state: DataChannelState,
    packet_at_hand: Option<Box<dyn Serializable>>,
}

impl DataChannel {
    /// Makes a new TFTPDataChannel with is backed by a File that's open
    /// in either read or write modes. If opening the File fails, an Error
    /// is returned.
    ///
    /// * `file_name` - Specified file name to read data from / write data to.
    /// * `channel_mode` - Tells whether this channel will be receiving or sending data.
    pub fn new(file_name: &str, channel_mode: DataChannelMode) -> Result<Self, ErrorPacket> {
        let (initial_blk, initial_state, fd) = match channel_mode {
            DataChannelMode::Tx => (
                -1,
                DataChannelState::SendData,
                File::open(Path::new(file_name)),
            ),
            DataChannelMode::Rx => (
                0,
                DataChannelState::WaitData,
                File::create(Path::new(file_name)),
            ),
        };

        if fd.is_err() {
            return Err(ErrorPacket::new(TFTPError::FileNotFound));
        }

        let mut channel = DataChannel {
            fd: fd.unwrap(),
            bytes: 0,
            blk: initial_blk,
            error: None,
            state: initial_state,
            packet_at_hand: None,
        };

        if channel.state == DataChannelState::SendData {
            channel.send_data();
        }

        Ok(channel)
    }

    /// Receives a data packet and checks its block number,
    /// if the packets block number is invalid an ErrorPacket is
    /// buffered, otherwise an AckPacket is buffered.
    ///
    /// * `dp` - Data packet received from the other end.
    pub fn on_data(&mut self, dp: DataPacket) {
        assert_eq!(self.state, DataChannelState::WaitData);

        if (self.blk + 1) as u16 != dp.blk() {
            self.set_blk_error(dp.blk());
            return;
        }

        self.blk = dp.blk() as isize;
        let data = &dp.data();
        self.bytes += data.len();
        self.fd.write_all(data).unwrap();

        if data.len() == STRIDE_SIZE {
            self.set_state(DataChannelState::SendAck);
        } else {
            self.set_state(DataChannelState::SendLastAck);
        }

        self.send_ack();
    }

    /// Receives an ACK packet from the server
    /// validates the block number then sends
    /// the next data block.
    pub fn on_ack(&mut self, ap: AckPacket) {
        assert!(
            self.state == DataChannelState::WaitAck || self.state == DataChannelState::WaitLastAck
        );

        if self.blk as u16 != ap.blk() {
            self.set_blk_error(ap.blk());
            return;
        }

        // TODO: fix upload
        self.blk += 1;

        match self.state {
            DataChannelState::WaitAck => {
                self.set_state(DataChannelState::SendData);
                self.send_data();
            }
            DataChannelState::WaitLastAck => {
                self.set_state(DataChannelState::Done);
            }
            _ => panic!("Should be waiting for am ACK."),
        }
    }

    fn send_ack(&mut self) {
        assert!(
            self.state == DataChannelState::SendAck || self.state == DataChannelState::SendLastAck
        );

        self.set_next_ack(AckPacket::new(self.blk as u16));

        if self.state == DataChannelState::SendLastAck {
            self.set_state(DataChannelState::Done);
        } else {
            self.set_state(DataChannelState::WaitData);
        }
    }

    /// Reads the next data packet to be sent,
    /// if this is the last packet, done will be
    /// set to true.
    fn send_data(&mut self) {
        assert_eq!(self.state, DataChannelState::SendData);

        self.blk += 1;

        let mut buf = [0; STRIDE_SIZE];
        let bytes_read = self.fd.read(&mut buf).unwrap();
        self.bytes += bytes_read;

        // When I read 0 bytes, this means that the client
        // just sent the ack for the last chunk in the file.
        if bytes_read == 0 {
            self.set_state(DataChannelState::WaitLastAck);
        } else {
            let data = Vec::from(&buf[0..bytes_read]);
            self.set_next_data(DataPacket::new(self.blk as u16, data));
            self.set_state(DataChannelState::WaitAck);
        }
    }

    fn set_state(&mut self, state: DataChannelState) {
        self.state = state;
    }

    fn set_blk_error(&mut self, actual: u16) {
        self.set_next_err(ErrorPacket::new(TFTPError::IllegalOperation));
        self.set_state(DataChannelState::Error);

        let err = format!(
            "Invalid block number [{}] expected [{}]",
            actual,
            self.blk + 1
        );
        self.error = Some(err);
    }

    fn set_next_data(&mut self, packet: DataPacket) {
        self.set_packet(Box::new(packet))
    }

    fn set_next_err(&mut self, packet: ErrorPacket) {
        self.set_packet(Box::new(packet))
    }

    fn set_next_ack(&mut self, packet: AckPacket) {
        self.set_packet(Box::new(packet))
    }
    fn set_packet(&mut self, packet: Box<dyn Serializable>) {
        self.packet_at_hand = Some(packet)
    }

    pub fn transfer_size(&self) -> usize {
        self.bytes
    }

    pub fn is_done(&self) -> bool {
        self.state == DataChannelState::Done
    }

    pub fn blk(&self) -> u16 {
        self.blk as u16
    }

    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    pub fn err(self) -> String {
        self.error.unwrap()
    }

    pub fn packet_at_hand(&mut self) -> Option<Box<dyn Serializable>> {
        mem::replace(&mut self.packet_at_hand, None)
    }
}
