use crate::tftp::common::{OP_RRQ, Packet, TFTPPacket};
use crate::tftp::common::OP_LEN;

#[derive(Debug)]
pub struct RequestPacket<'a> {
    filename: &'a str,
    mode: &'a str,
    op: u16,
}

impl<'a> RequestPacket<'a> {
    pub fn new(op: u16, filename: &'a str, mode: &'a str) -> Self {
        RequestPacket {
            op,
            filename,
            mode,
        }
    }
}

impl<'a> Packet for RequestPacket<'a> {
    fn into_bytes(self) -> Vec<u8> {
        let length =
            OP_LEN +
                self.filename.len() +
                self.mode.len();
        let mut buf = Vec::with_capacity(length);
        self.serialize_op(&mut buf);
        buf.append(&mut Vec::from(self.filename.as_bytes()));
        buf.push(0);
        buf.append(&mut Vec::from(self.mode.as_bytes()));
        buf.push(0);
        buf
    }
}

impl<'a> TFTPPacket for RequestPacket<'a> {
    fn op(&self) -> u16 {
        return self.op;
    }

    fn serialize_op(&self, buf: &mut Vec<u8>) {
        buf.push((self.op() & 0xFF00) as u8);
        buf.push((self.op() & 0x00FF) as u8);
    }
}