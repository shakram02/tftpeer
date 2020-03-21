use std::io::Write;
use std::str;

use byteorder::NetworkEndian;

use crate::tftp::shared::{
    Deserializable, Serializable, TFTPPacket, TFTPParseError, OP_LEN, OP_RRQ, OP_WRQ,
};

use super::byteorder::{ByteOrder, WriteBytesExt};

pub trait Request: Serializable + Deserializable {
    fn op(&self) -> u16;
    fn filename(&self) -> &str;
    fn mode(&self) -> &str;
}

#[derive(Debug, Eq, PartialEq)]
pub struct ReadRequestPacket {
    req: RequestPacket,
}

impl ReadRequestPacket {
    pub fn new(filename: &str, mode: &str) -> ReadRequestPacket {
        ReadRequestPacket {
            req: RequestPacket::new(OP_RRQ, filename, mode),
        }
    }
}

impl Request for ReadRequestPacket {
    fn op(&self) -> u16 {
        self.req.op
    }

    fn filename(&self) -> &str {
        &self.req.filename
    }

    fn mode(&self) -> &str {
        &self.req.mode
    }
}

impl Serializable for ReadRequestPacket {
    fn serialize(self) -> Vec<u8> {
        self.req.serialize()
    }
}

impl Deserializable for ReadRequestPacket {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        RequestPacket::deserialize(buf)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteRequestPacket {
    req: RequestPacket,
}

impl WriteRequestPacket {
    pub fn new(filename: &str, mode: &str) -> WriteRequestPacket {
        WriteRequestPacket {
            req: RequestPacket::new(OP_WRQ, filename, mode),
        }
    }
}

impl Request for WriteRequestPacket {
    fn op(&self) -> u16 {
        self.req.op
    }

    fn filename(&self) -> &str {
        &self.req.filename
    }

    fn mode(&self) -> &str {
        &self.req.mode
    }
}

impl Serializable for WriteRequestPacket {
    fn serialize(self) -> Vec<u8> {
        self.req.serialize()
    }
}

impl Deserializable for WriteRequestPacket {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        RequestPacket::deserialize(buf)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct RequestPacket {
    op: u16,
    filename: String,
    mode: String,
}

impl RequestPacket {
    fn new(op: u16, filename: &str, mode: &str) -> Self {
        RequestPacket {
            op,
            filename: String::from(filename),
            mode: String::from(mode),
        }
    }
}

impl Serializable for RequestPacket {
    fn serialize(self) -> Vec<u8> {
        let length = OP_LEN + self.filename.len() + self.mode.len();
        let mut buf = Vec::with_capacity(length);
        // self.serialize_op(&mut buf);

        buf.write_u16::<NetworkEndian>(self.op).unwrap();
        buf.write_all(self.filename.as_bytes()).unwrap();
        buf.write_u8(0).unwrap();
        buf.write_all(self.mode.as_bytes()).unwrap();
        buf.write_u8(0).unwrap();
        buf
    }
}

impl Deserializable for RequestPacket {
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        // TODO: add options

        let op: u16 = NetworkEndian::read_u16(&buf[0..2]);
        if ![OP_RRQ, OP_WRQ].contains(&op) {
            return Err(TFTPParseError::new("Bad OP code!"));
        }

        let buf = &buf[2..];
        let mut data: Vec<&str> = buf
            .split(|&byte| byte == 0)
            .map(|item| str::from_utf8(item).unwrap())
            .filter(|s| s.len() != 0)
            .collect();

        let filename = data.remove(0);
        let mode = data.remove(0);

        let packet = match op {
            OP_RRQ => TFTPPacket::RRQ(ReadRequestPacket::new(filename, mode)),
            OP_WRQ => TFTPPacket::WRQ(WriteRequestPacket::new(filename, mode)),
            _ => panic!("Invalid op code."),
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use crate::tftp::shared::request_packet::{Request, RequestPacket};
    use crate::tftp::shared::{
        Deserializable, Serializable, TFTPPacket, TFTPParseError, OP_RRQ, OP_WRQ,
    };

    const FILE_NAME: &str = "a.txt";
    const MODE: &str = "octet";

    #[test]
    fn serialize_rrq() {
        let p = RequestPacket::new(OP_RRQ, FILE_NAME, MODE);
        let bytes: Vec<u8> = vec![
            0x0, 0x1, 0x61, 0x2E, 0x74, 0x78, 0x74, 0x0, 0x6F, 0x63, 0x74, 0x65, 0x74, 0x0,
        ];
        assert_eq!(bytes, p.serialize());
    }

    #[test]
    fn serialize_wrq() {
        let p = RequestPacket::new(OP_WRQ, FILE_NAME, MODE);
        let bytes: Vec<u8> = vec![
            0x0, 0x2, 0x61, 0x2E, 0x74, 0x78, 0x74, 0x0, 0x6F, 0x63, 0x74, 0x65, 0x74, 0x0,
        ];
        assert_eq!(bytes, p.serialize());
    }

    #[test]
    fn deserialize_rrq() {
        let mut bytes: Vec<u8> = vec![
            0x0, 0x1, 0x61, 0x2E, 0x74, 0x78, 0x74, 0x0, 0x6F, 0x63, 0x74, 0x65, 0x74, 0x0,
        ];

        if let TFTPPacket::RRQ(p) = RequestPacket::deserialize(&mut bytes).unwrap() {
            assert_eq!(p.op(), OP_RRQ);
            assert_eq!(p.filename(), "a.txt");
            assert_eq!(p.mode(), "octet");
        } else {
            panic!("Wrong packet type")
        }
    }

    #[test]
    fn deserialize_bad_op() {
        let mut bytes: Vec<u8> = vec![
            0x0, 0x61, 0x2E, 0x74, 0x78, 0x74, 0x0, 0x6F, 0x63, 0x74, 0x65, 0x74, 0x0,
        ];
        let p = RequestPacket::deserialize(&mut bytes).err().unwrap();
        assert_eq!(p, TFTPParseError::new("Bad OP code!"));
    }
}
