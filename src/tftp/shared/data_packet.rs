use std::io::Write;

use crate::tftp::shared::{
    Deserializable, Serializable, TFTPPacket, TFTPParseError, OP_DATA, OP_LEN,
};

use super::byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

const BLK_NUM_LEN: usize = 2;
const DATA_MAX_LEN: usize = 512;

#[derive(Debug, Eq, PartialEq)]
pub struct DataPacket {
    op: u16,
    blk: u16,
    data: Vec<u8>,
}

impl DataPacket {
    pub fn new(blk: u16, data: Vec<u8>) -> Self {
        DataPacket {
            op: OP_DATA,
            blk,
            data,
        }
    }

    pub fn blk(&self) -> u16 {
        self.blk
    }
    pub fn data(self) -> Vec<u8> {
        self.data
    }
}

impl DataPacket {
    fn data_length(&self) -> usize {
        self.data.len()
    }
}

impl Serializable for DataPacket {
    fn box_serialize(self: Box<Self>) -> Vec<u8> {
        self.serialize()
    }

    fn serialize(self) -> Vec<u8> {
        let buf_len = OP_LEN + BLK_NUM_LEN + self.data_length();
        let mut buf: Vec<u8> = Vec::with_capacity(buf_len);
        // self.serialize_op(&mut buf);
        buf.write_u16::<NetworkEndian>(self.op).unwrap();
        buf.write_u16::<NetworkEndian>(self.blk).unwrap();
        buf.write_all(self.data.as_slice()).unwrap();

        buf
    }
}

impl Deserializable for DataPacket {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        let op: u16 = NetworkEndian::read_u16(&buf[0..2]);

        if OP_DATA != op {
            return Err(TFTPParseError::new("Bad OP code!"));
        }

        let blk = NetworkEndian::read_u16(&buf[2..4]);
        let data = &buf[4..];

        if data.len() > DATA_MAX_LEN {
            return Err(TFTPParseError::new("Invalid data length"));
        }

        let p = DataPacket::new(blk, data.to_vec());
        Ok(TFTPPacket::DATA(p))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn serialize_data_packet() {}

    #[test]
    fn deserialize_data_packet() {}

    #[test]
    fn deserialize_error() {}
}
