pub mod request_packet;

/// Length of the OpCode field in bytes.
pub(self) const OP_LEN: usize = 2;

/// Total length of separators in bytes in a request packet.
const REQUEST_SEP_LENGTH: usize = 2;

/// Op code for Read Request
pub const OP_RRQ: u16 = 0x001;
/// Op code for Write Request
pub const OP_WRQ: u16 = 0x002;
/// Op code for Data packet
pub const OP_DATA: u16 = 0x003;
/// Op code for ACK packet
pub const OP_ACK: u16 = 0x004;
/// Op code for Error packet
pub const OP_ERR: u16 = 0x005;

pub trait Packet {
    fn into_bytes(self) -> Vec<u8>;
}

pub trait TFTPPacket: Packet {
    fn op(&self) -> u16;
    fn serialize_op(&self, buf: &mut Vec<u8>);
}
