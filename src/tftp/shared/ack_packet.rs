/// ACK packets are acknowledged by  DATA  or ERROR packets.
/// the opcode is 4.
///
/// The  block  number  in an  ACK echoes
/// the block number of the DATA packet being acknowledged.
///
/// A WRQ is acknowledged with an ACK packet having a
/// block number of zero.
use crate::tftp::shared::{Deserializable, Serializable, TFTPPacket, TFTPParseError, OP_ACK};

use super::byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

const ACK_LEN: usize = 4;
const BLK_NUM_OFFSET: usize = 2;

#[derive(Debug, Eq, PartialEq)]
pub struct AckPacket {
    op: u16,
    blk: u16,
}

impl AckPacket {
    pub fn new(blk: u16) -> Self {
        AckPacket { op: OP_ACK, blk }
    }

    pub fn blk(&self) -> u16 {
        self.blk
    }
}

impl Serializable for AckPacket {
    fn serialize(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(ACK_LEN);
        buf.write_u16::<NetworkEndian>(self.op).unwrap();
        buf.write_u16::<NetworkEndian>(self.blk).unwrap();

        buf
    }
}

impl Deserializable for AckPacket {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        let op = NetworkEndian::read_u16(buf);

        if op != OP_ACK {
            return Err(TFTPParseError::new(
                format!("Bad OP code! [{}]", op).as_str(),
            ));
        }

        let blk = NetworkEndian::read_u16(&buf[BLK_NUM_OFFSET..]);
        Ok(TFTPPacket::ACK(AckPacket::new(blk)))
    }
}

#[cfg(test)]
mod tests {
    use crate::tftp::shared::ack_packet::AckPacket;
    use crate::tftp::shared::{Deserializable, Serializable, TFTPPacket, OP_ACK};

    use super::super::byteorder::{NetworkEndian, WriteBytesExt};

    #[test]
    fn serialize_ack_packet() {
        let blk = 42;
        let p = AckPacket::new(blk);

        let mut buf: Vec<u8> = Vec::new();
        buf.write_u16::<NetworkEndian>(OP_ACK).unwrap();
        buf.write_u16::<NetworkEndian>(blk).unwrap();

        assert_eq!(p.serialize(), buf);
    }

    #[test]
    fn deserialize_ack_packet() {
        let blk = 42;
        let mut buf: Vec<u8> = Vec::new();
        buf.write_u16::<NetworkEndian>(OP_ACK).unwrap();
        buf.write_u16::<NetworkEndian>(blk).unwrap();

        let p = AckPacket::new(blk);
        if let TFTPPacket::ACK(d) = AckPacket::deserialize(&mut buf).unwrap() {
            assert_eq!(d, p);
        }
    }

    #[test]
    fn deserialize_error() {
        let blk = 42;
        let bad_op = OP_ACK + 1;
        let mut buf: Vec<u8> = Vec::new();
        buf.write_u16::<NetworkEndian>(bad_op).unwrap();
        buf.write_u16::<NetworkEndian>(blk).unwrap();

        let p = AckPacket::deserialize(&mut buf).unwrap_err();
        assert_eq!(p.details, format!("Bad OP code! [{}]", bad_op).as_str())
    }
}
