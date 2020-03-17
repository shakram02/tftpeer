/// ERROR packet can be the acknowledgment of any other type of packet.
/// The error code is an integer indicating the nature of the error.  A
/// table of values and meanings is given in the appendix.  (Note that
/// several error codes have been added to this version of this
/// document.) The error message is intended for human consumption, and
/// should be in netascii.  Like all other strings, it is terminated with
/// a zero byte.

use std::io::Write;

use crate::tftp::common::{Deserializable, Serializable, TFTPPacket, TFTPParseError, OP_ERR};

use super::byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

const ERR_LEN: usize = 4;
const ERR_OFFSET: usize = 4;

#[derive(Debug, Eq, PartialEq)]
pub struct ErrorPacket {
    op: u16,
    code: u16,
    err: String,
}

impl ErrorPacket {
    fn new(code: u16, msg: &str) -> Self {
        ErrorPacket { op: OP_ERR, code, err: msg.to_string() }
    }
}

impl Serializable for ErrorPacket {
    fn serialize(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(ERR_LEN);
        buf.write_u16::<NetworkEndian>(self.op).unwrap();
        buf.write_u16::<NetworkEndian>(self.code).unwrap();
        let terminated_err_msg: String = if !self.err.ends_with("\0") {
            let mut err_msg = String::from(self.err);
            err_msg.push('\0');
            err_msg
        } else {
            self.err
        };

        buf.write_all(terminated_err_msg.as_bytes()).unwrap();
        buf
    }
}

impl Deserializable for ErrorPacket {
    fn deserialize(buf: &Vec<u8>) -> Result<TFTPPacket, TFTPParseError> {
        let op = NetworkEndian::read_u16(buf);

        if op != OP_ERR {
            return Err(TFTPParseError::new(format!("Bad OP code! [{}]", op).as_str()));
        }

        let code = NetworkEndian::read_u16(buf);
        let msg = std::str::from_utf8(&buf[ERR_OFFSET..]).unwrap();

        let p = ErrorPacket::new(code, msg);
        Ok(TFTPPacket::ERR(p))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::tftp::common::{Deserializable, Serializable, TFTPPacket, OP_ERR};
    use crate::tftp::common::err_packet::ErrorPacket;

    use super::super::byteorder::{NetworkEndian, WriteBytesExt};

    #[test]
    fn serialize_ack_packet() {
        let err_msg = "error message\0";
        let p = ErrorPacket::new(0, err_msg);
        let msg_bytes = &mut Vec::from(err_msg.as_bytes());
        let mut serialized = vec![0, 5, 0, 0];
        serialized.append(msg_bytes);

        assert_eq!(p.serialize(), serialized);
    }

    #[test]
    fn deserialize_ack_packet() {
        let err_msg = "error message\0";
        let err_code: u16 = 5;
        let mut buf = Vec::new();
        let msg_bytes = &mut Vec::from(err_msg.as_bytes());
        buf.write_u16::<NetworkEndian>(OP_ERR).unwrap();
        buf.write_u16::<NetworkEndian>(err_code).unwrap();
        buf.write_all(msg_bytes.as_slice()).unwrap();

        if let TFTPPacket::ERR(p) = ErrorPacket::deserialize(&mut buf).unwrap() {
            assert_eq!(p.op, OP_ERR);
            assert_eq!(p.code, err_code);
            assert_eq!(p.err, err_msg);
        } else { panic!("Invalid type") }
    }

    #[test]
    fn deserialize_error() {
        let err_msg = "error message\0";
        let err_code: u16 = 5;

        let mut buf = Vec::new();
        let msg_bytes = &mut Vec::from(err_msg.as_bytes());
        let bad_op = OP_ERR + 1;
        buf.write_u16::<NetworkEndian>(bad_op).unwrap();
        buf.write_u16::<NetworkEndian>(err_code).unwrap();
        buf.write_all(msg_bytes.as_slice()).unwrap();

        let p = ErrorPacket::deserialize(&mut buf).unwrap_err();
        assert_eq!(p.details, format!("Bad OP code! [{}]", bad_op).as_str())
    }
}
