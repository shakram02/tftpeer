extern crate pretty_bytes;

use std::fs::File;
use std::io::{Read, Write};
use std::net::UdpSocket;
use std::path::Path;

use pretty_bytes::converter::convert;

use crate::tftp::shared::{Serializable, STRIDE_SIZE, TFTPPacket};
use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};
use crate::tftp::shared::request_packet::{ReadRequestPacket, WriteRequestPacket};

/// A TFTP server that supports a single client.
struct TFTPServer {
    // TODO
}
