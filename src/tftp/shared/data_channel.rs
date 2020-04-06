use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::io;
use std::path::Path;

use crate::tftp::shared::{Serializable, STRIDE_SIZE};
use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum DataChannelMode {
    Tx,
    Rx,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum DataChannelOwner {
    Server,
    Client,
}

pub struct DataChannel {
    fd: Option<File>,
    file_name: String,
    bytes: usize,
    blk: u16,
    error: Option<String>,
    state: DataChannelState,
    owner: DataChannelOwner,
    packet_at_hand: Option<Vec<u8>>,
}

impl DataChannel {
    /// Makes a new TFTPDataChannel with is backed by a File that's open
    /// in either read or write modes. If opening the File fails, an Error
    /// is returned.
    ///
    /// * `file_name` - Specified file name to read data from / write data to.
    /// * `channel_mode` - Tells whether this channel will be receiving or sending data.
    pub fn new(file_name: &str, mode: DataChannelMode, owner: DataChannelOwner) -> Result<Self, ErrorPacket> {
        let (initial_blk, initial_state) =
            DataChannel::compute_initial_state(mode, owner);

        let maybe_fd = if mode == DataChannelMode::Tx {
            let fd = DataChannel::open_file_for_transmission(file_name, owner);
            if let Err(ep) = fd {
                return Err(ep);
            }

            Some(fd.unwrap())
        } else {
            let fp_valid = DataChannel::validate_file_for_reception(file_name, owner);
            if let Err(ep) = fp_valid {
                return Err(ep);
            }

            None
        };

        let mut channel = DataChannel {
            fd: maybe_fd,
            file_name: file_name.to_string(),
            bytes: 0,
            blk: initial_blk,
            error: None,
            state: initial_state,
            owner,
            packet_at_hand: None,
        };


        if channel.state == DataChannelState::SendData {
            channel.send_data();
        } else if channel.state == DataChannelState::SendAck {
            channel.send_ack();
        }

        Ok(channel)
    }

    fn compute_initial_state(channel_mode: DataChannelMode, channel_owner: DataChannelOwner) -> (u16, DataChannelState) {
        match channel_mode {
            DataChannelMode::Tx => {
                if channel_owner == DataChannelOwner::Client {
                    // An uploading client will be waiting for ACK #0
                    (0, DataChannelState::WaitAck)
                } else {
                    // A server sending data will start with DATA #1
                    // do_data() increases the block number anyways.
                    (0, DataChannelState::SendData)
                }
            }
            DataChannelMode::Rx => {
                if channel_owner == DataChannelOwner::Client {
                    // A downloading client will wait for DATA 1
                    (1, DataChannelState::WaitData)
                } else {
                    // A server receiving data will be sending ACK 0.
                    (0, DataChannelState::SendAck)
                }
            }
        }
    }

    fn open_file_for_transmission(file_name: &str, owner: DataChannelOwner) -> Result<File, ErrorPacket> {
        use std::fs;
        let fp = Path::new(file_name);
        let fd = File::open(fp)
            .and_then(|fd| {
                let meta = fs::metadata(fp).unwrap();
                if meta.len() == 0 {
                    let direction = if owner == DataChannelOwner::Server {
                        "Requested"
                    } else {
                        "Transmitted"
                    };
                    let msg = format!("{} file is empty.", direction);
                    Err(Error::new(ErrorKind::InvalidData, msg))
                } else {
                    Ok(fd)
                }
            });

        if fd.is_err() {
            let err = fd.unwrap_err();

            return if err.kind() == ErrorKind::NotFound {
                Err(ErrorPacket::new(TFTPError::FileNotFound))
            } else {
                Err(ErrorPacket::new_custom(err.to_string()))
            };
        }

        Ok(fd.unwrap())
    }

    fn validate_file_for_reception(file_name: &str, owner: DataChannelOwner) -> Result<(), ErrorPacket> {
        let path = Path::new(file_name);

        if Path::exists(path) && owner == DataChannelOwner::Server {
            return Err(ErrorPacket::new(TFTPError::FileExists));
        }

        if Path::file_name(path) == None || path.is_dir() {
            let err = String::from("Can't write a directory");
            return Err(ErrorPacket::new_custom(err));
        }

        // Client isn't allowed to traverse the TFTP directory upwards
        // in any case.
        if file_name.contains("..") {
            let err = String::from("Only absolute paths are allowed.");
            return Err(ErrorPacket::new_custom(err));
        }

        // Client needn't know anything about the server's host.
        if path.is_absolute() {
            let err = String::from("File path must not start with root.");
            return Err(ErrorPacket::new_custom(err));
        }

        // File to be added is a decedent of the TFTP server directory.
        if path.is_relative() && path.parent() != None {
            use std::fs;
            if let Err(e) = fs::create_dir_all(path.parent().unwrap()) {
                return Err(ErrorPacket::new_custom(e.to_string()));
            }
        }

        Ok(())
    }

    /// Receives a data packet and checks its block number,
    /// if the packets block number is invalid an ErrorPacket is
    /// buffered, otherwise an AckPacket is buffered.
    ///
    /// * `dp` - Data packet received from the other end.
    pub fn on_data(&mut self, dp: DataPacket) {
        assert_eq!(self.state, DataChannelState::WaitData);
        println!("ON_DATA #{:?}", dp.blk());

        // The received blk
        // is the awaited blk number.
        if (self.blk + 1) as u16 != dp.blk() {
            self.set_blk_error(dp.blk());
            return;
        }

        // To avoid making empty files needlessly.
        if dp.blk() == 1 {
            let fp = Path::new(&self.file_name);
            self.fd = Some(File::create(fp).unwrap());
        }

        // The party the receives data, sets
        // its block number to ACK it on the
        // text transfer.
        self.blk = dp.blk();
        let data = &dp.data();
        self.bytes += data.len();
        self.fd.as_ref().unwrap().write_all(data).unwrap();

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
        let bytes_read = self.fd.as_ref().unwrap().read(&mut buf).unwrap();
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
        println!("STATE: {:?}", self.state);
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
        self.set_err(&err);
    }

    fn set_err(&mut self, msg: &str) {
        self.error = Some(msg.to_string());
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
