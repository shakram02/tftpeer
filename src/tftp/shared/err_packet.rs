/// ERROR packet can be the acknowledgment of any other type of packet.
/// The error code is an integer indicating the nature of the error.  A
/// table of values and meanings is given in the appendix.  (Note that
/// several error codes have been added to this version of this
/// document.) The error message is intended for human consumption, and
/// should be in netascii.  Like all other strings, it is terminated with
/// a zero byte.
use std::io::Write;

use crate::tftp::shared::{Deserializable, OP_ERR, Serializable, TFTPPacket, TFTPParseError};

use super::byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

const ERR_LEN: usize = 4;

#[derive(Debug, Eq, PartialEq)]
pub struct ErrorPacket {
    op: u16,
    code: u16,
    err: String,
}

pub enum TFTPError {
    UndefinedError,
    FileNotFound,
    AccessViolation,
    DiskFull,
    IllegalOperation,
    UnknownTID,
    FileExists,
}

fn get_err_by_code(code: u16) -> (TFTPError, String) {
    match code {
        0 => (
            TFTPError::UndefinedError,
            String::from("Not defined, see error message (if any).\0"),
        ),
        1 => (TFTPError::FileNotFound, String::from("File not found.\0")),
        2 => (
            TFTPError::AccessViolation,
            String::from("Access violation.\0"),
        ),
        3 => (
            TFTPError::DiskFull,
            String::from("Disk full or allocation exceeded.\0"),
        ),
        4 => (
            TFTPError::IllegalOperation,
            String::from("Illegal TFTP operation.\0"),
        ),
        5 => (
            TFTPError::UnknownTID,
            String::from("Unknown transfer ID.\0"),
        ),
        6 => (
            TFTPError::FileExists,
            String::from("File already exists.\0"),
        ),
        _ => panic!(format!("Invalid error code [{}]", code)),
    }
}

fn get_err_details(err: TFTPError) -> (u16, String) {
    match err {
        TFTPError::UndefinedError => (
            0,
            String::from("Not defined, see error message (if any).\0"),
        ),
        TFTPError::FileNotFound => (1, String::from("File not found.\0")),
        TFTPError::AccessViolation => (2, String::from("Access violation.\0")),
        TFTPError::DiskFull => (3, String::from("Disk full or allocation exceeded.\0")),
        TFTPError::IllegalOperation => (4, String::from("Illegal TFTP operation.\0")),
        TFTPError::UnknownTID => (5, String::from("Unknown transfer ID.\0")),
        TFTPError::FileExists => (6, String::from("File already exists.\0")),
        TFTPError::NoSuchUser => (7, String::from("No such user.\0")),
    }
}

impl ErrorPacket {
    pub fn new(err: TFTPError) -> Self {
        let (code, msg) = get_err_details(err);
        ErrorPacket {
            op: OP_ERR,
            code,
            err: msg,
        }
    }

    pub fn code(&self) -> u16 {
        self.code
    }
    pub fn err(&self) -> &str {
        &self.err
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
    fn deserialize(buf: &[u8]) -> Result<TFTPPacket, TFTPParseError> {
        let op = NetworkEndian::read_u16(buf);

        if op != OP_ERR {
            return Err(TFTPParseError::new(
                format!("Bad OP code! [{}]", op).as_str(),
            ));
        }

        let code = NetworkEndian::read_u16(buf);
        let (err_type, _) = get_err_by_code(code);

        let p = ErrorPacket::new(err_type);
        Ok(TFTPPacket::ERR(p))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::tftp::shared::{Deserializable, OP_ERR, Serializable, TFTPPacket};
    use crate::tftp::shared::err_packet::{ErrorPacket, get_err_details};
    use crate::tftp::shared::err_packet::TFTPError::IllegalOperation;

    use super::super::byteorder::{NetworkEndian, WriteBytesExt};

    #[test]
    fn serialize_ack_packet() {
        let p = ErrorPacket::new(IllegalOperation);
        let (code, err) = get_err_details(IllegalOperation);
        let msg_bytes = &mut Vec::from(err.as_bytes());
        let mut serialized = vec![0, 5, 0, code as u8];
        serialized.append(msg_bytes);

        assert_eq!(p.serialize(), serialized);
    }

    #[test]
    fn deserialize_ack_packet() {
        let (err_code, err_msg) = get_err_details(IllegalOperation);
        let mut buf = Vec::new();
        let msg_bytes = &mut Vec::from(err_msg.as_bytes());
        buf.write_u16::<NetworkEndian>(OP_ERR).unwrap();
        buf.write_u16::<NetworkEndian>(err_code).unwrap();
        buf.write_all(msg_bytes.as_slice()).unwrap();

        if let TFTPPacket::ERR(p) = ErrorPacket::deserialize(&mut buf).unwrap() {
            assert_eq!(p.op, OP_ERR);
            assert_eq!(p.code, err_code);
            assert_eq!(p.err, err_msg);
        } else {
            panic!("Invalid type")
        }
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
