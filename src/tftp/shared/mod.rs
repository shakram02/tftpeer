extern crate byteorder;

use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::ErrorPacket;
use crate::tftp::shared::request_packet::*;

use self::byteorder::{ByteOrder, NetworkEndian};

pub mod ack_packet;
pub mod data_channel;
pub mod data_packet;
pub mod err_packet;
pub mod request_packet;

const OP_LEN: usize = 2;
/// Stride size for reading / writing files.
pub const STRIDE_SIZE: usize = 512;
/// Op code for Data packet
const OP_DATA: u16 = 0x003;
/// Op code for Read Request
const OP_RRQ: u16 = 0x001;
/// Op code for Write Request
const OP_WRQ: u16 = 0x002;
/// Op code for Error packet
const OP_ERR: u16 = 0x005;
/// Op code for ACK packet
const OP_ACK: u16 = 0x004;

#[derive(Debug, Eq, PartialEq)]
pub enum TFTPPacket {
    RRQ(ReadRequestPacket),
    WRQ(WriteRequestPacket),
    ACK(AckPacket),
    ERR(ErrorPacket),
    DATA(DataPacket),
}

impl Display for TFTPPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let desc = match self {
            TFTPPacket::RRQ(p) => format!("RRQ [{}] [{}]", p.filename(), p.mode()),
            TFTPPacket::WRQ(p) => format!("WRQ [{}] [{}]", p.filename(), p.mode()),
            TFTPPacket::ACK(p) => format!("ACK [{}]", p.blk()),
            TFTPPacket::ERR(p) => format!("ERR [{}]: {}", p.code(), p.err()),
            TFTPPacket::DATA(p) => format!("DATA [{}]", p.blk()),
        };

        write!(f, "{}", desc)
    }
}

pub trait Serializable {
    fn box_serialize(self: Box<Self>) -> Vec<u8>;
    fn serialize(self) -> Vec<u8>;
}

pub trait Deserializable {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError>;
}

pub fn parse_udp_packet(buf: &[u8]) -> TFTPPacket {
    let p = match NetworkEndian::read_u16(buf) {
        OP_RRQ => ReadRequestPacket::deserialize(buf),
        OP_WRQ => WriteRequestPacket::deserialize(buf),
        OP_ACK => AckPacket::deserialize(buf),
        OP_ERR => ErrorPacket::deserialize(buf),
        OP_DATA => DataPacket::deserialize(buf),
        val => panic!(format!("Invalid opcode [{}]", val)),
    };

    p.unwrap()
}

#[derive(Debug, Eq, PartialEq)]
pub struct TFTPParseError {
    details: String,
}

impl Error for TFTPParseError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl TFTPParseError {
    fn new(msg: &str) -> TFTPParseError {
        TFTPParseError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for TFTPParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse packet: {}", self.details)
    }
}
