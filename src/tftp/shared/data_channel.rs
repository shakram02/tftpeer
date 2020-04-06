use std::fs::File;
use std::io::{Read, Write};
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

pub enum DataChannelOwner {
    Server,
    Client,
}

pub struct DataChannel {
    fd: File,
    bytes: usize,
    blk: isize,
    error: Option<String>,
    state: DataChannelState,
    packet_at_hand: Option<Vec<u8>>,
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
            // We an RRQ is received, we go to SEND_DATA
            // state to send the DATA #1.
            DataChannelMode::Tx => (
                0,
                DataChannelState::SendData,
                File::open(Path::new(file_name)),
            ),
            // We an WRQ is received, we go to SEND_ACK
            // state to send the ACK #0.
            DataChannelMode::Rx => (
                0,
                DataChannelState::SendAck,
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
        } else {
            // ACK for RRQ.
            channel.send_ack();
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
        println!("ON_DATA #{:?}", dp.blk());

        if (self.blk + 1) as u16 != dp.blk() {
            self.set_blk_error(dp.blk());
            return;
        }

        // The party the receives data, sets
        // its block number to ACK it on the
        // text transfer.
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

    fn send_ack(&mut self) {
        assert!(
            self.state == DataChannelState::SendAck || self.state == DataChannelState::SendLastAck
        );
        println!("DO_ACK #{:?}", self.blk);

        self.set_next_ack(AckPacket::new(self.blk as u16));

        if self.state == DataChannelState::SendAck {
            self.set_state(DataChannelState::WaitData);
        }
    }

    /// Reads the next data packet to be sent,
    /// if this is the last packet, done will be
    /// set to true.
    fn send_data(&mut self) {
        assert_eq!(self.state, DataChannelState::SendData);
        println!("DO_DATA #{:?}", self.blk + 1);

        self.blk += 1;

        let mut buf = [0; STRIDE_SIZE];
        let bytes_read = self.fd.read(&mut buf).unwrap();
        self.bytes += bytes_read;

        // When I read 0 bytes, this means that the client
        // just sent the ack for the last chunk in the file.
        if bytes_read == 0 {
            self.set_state(DataChannelState::Done);
            return; // Don't prepare any data packets, we're done.
        } else if bytes_read < STRIDE_SIZE {
            self.set_state(DataChannelState::WaitLastAck);
        } else {
            self.set_state(DataChannelState::WaitAck);
        }

        // Send the next data packet.
        let data = Vec::from(&buf[0..bytes_read]);
        self.set_next_data(DataPacket::new(self.blk as u16, data));
    }

    /// Receives an ACK packet from the server
    /// validates the block number then sends
    /// the next data block.
    pub fn on_ack(&mut self, ap: AckPacket) {
        assert!(
            self.state == DataChannelState::WaitAck || self.state == DataChannelState::WaitLastAck
        );
        println!("ON_ACK #{:?}", ap.blk());

        if self.blk as u16 != ap.blk() {
            self.set_blk_error(ap.blk());
            return;
        }

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

    fn set_state(&mut self, state: DataChannelState) {
        println!("Moving to {:?}", state);
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
        println!("DATA_AT_HAND #{}", packet.blk());
        self.set_packet(packet.serialize());
    }

    fn set_next_err(&mut self, packet: ErrorPacket) {
        self.set_packet(packet.serialize());
    }

    fn set_next_ack(&mut self, packet: AckPacket) {
        println!("ACK_AT_HAND #{}", packet.blk());
        self.set_packet(packet.serialize());
    }

    fn set_packet(&mut self, packet: Vec<u8>) {
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

    pub fn packet_at_hand(&mut self) -> Option<Vec<u8>> {
        assert_ne!(self.state, DataChannelState::Done);
        // If the previous state was SendLastAck,
        // now we're done.
        if self.state == DataChannelState::SendLastAck {
            self.set_state(DataChannelState::Done);
        }

        match &self.packet_at_hand {
            None => None,
            Some(p) => {
                Some(p.clone())
            }
        }
    }
}
