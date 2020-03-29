extern crate pretty_bytes;



use std::net::UdpSocket;

use pretty_bytes::converter::convert;

use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};
use crate::tftp::shared::request_packet::{ReadRequestPacket, WriteRequestPacket};
use crate::tftp::shared::{Serializable, TFTPPacket};

/// A TFTP server that supports a single client.
struct TFTPServer {
    // TODO
}
