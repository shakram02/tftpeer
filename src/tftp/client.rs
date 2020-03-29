extern crate pretty_bytes;

use std::net::UdpSocket;

use pretty_bytes::converter::convert;

use crate::tftp::shared::{Serializable, TFTPPacket};
use crate::tftp::shared::ack_packet::AckPacket;
use crate::tftp::shared::data_packet::DataPacket;
use crate::tftp::shared::err_packet::ErrorPacket;
use crate::tftp::shared::request_packet::{ReadRequestPacket, WriteRequestPacket};
use crate::tftp::shared::tftp_data_channel::{DataChannelMode, TFTPDataChannel};

struct TFTPClient {
    packet_buffer: Vec<Vec<u8>>,
    data_channel: TFTPDataChannel,
    error: Option<String>,
}

impl TFTPClient {
    /// Constructs a new TFTPClient.
    fn new(file_name: &str, mode: DataChannelMode) -> Self {
        let data_channel = TFTPDataChannel::new(file_name, mode).unwrap();
        // Keep the information we need to know
        // in the object and initialize them
        // to some default values.
        TFTPClient {
            packet_buffer: Vec::new(),
            data_channel,
            error: None,
        }
    }

    /// Places a RRQ in the packet buffer to be sent to the server.
    pub fn download(file_name: &str) -> TFTPClient {
        let mut client = TFTPClient::new(file_name, DataChannelMode::Rx);

        let rrq = ReadRequestPacket::new(file_name, "octet");
        client.packet_buffer.push(rrq.serialize());

        client
    }

    /// Places a WRQ in the packet buffer to be sent
    /// to the server, then opens the file to be read.
    pub fn upload(file_name: &str) -> TFTPClient {
        let mut client = TFTPClient::new(file_name, DataChannelMode::Tx);

        let wrq = WriteRequestPacket::new(file_name, "octet");
        client.packet_buffer.push(wrq.serialize());

        client
    }

    /// Returns the first packet in the packet
    /// buffer to be sent to the server.
    pub fn get_next_packet(&mut self) -> Vec<u8> {
        self.packet_buffer.remove(0)
    }

    /// Tells whether the client's packet buffer
    /// has any pending packets to be sent.
    pub fn has_packets(&self) -> bool {
        self.packet_buffer.len() != 0
    }

    /// Facade to client logic, parses the given buffer to a TFTP packet
    /// then acts accordingly.
    pub fn process_packet(&mut self, buf: &[u8]) {
        let packet = crate::tftp::shared::parse_udp_packet(&buf);
        match packet {
            TFTPPacket::DATA(data) => self.on_data(data),
            TFTPPacket::ACK(ack) => self.on_ack(ack),
            TFTPPacket::ERR(err) => self.on_err(err),
            t => panic!(format!("Unexpected packet type: [{:?}]", t)),
        };
    }

    /// Receive a data packet.
    /// Validate the block number then write
    /// the data received to file.
    fn on_data(&mut self, dp: DataPacket) {
        let dp_blk = dp.blk();
        let packet = match self.data_channel.receive_data(dp) {
            Ok(ack_packet) => ack_packet.serialize(),
            Err(error_packet) => {
                self.set_blk_error(dp_blk);
                error_packet.serialize()
            }
        };

        self.packet_buffer.push(packet);
        println!(
            "Received [{}]",
            convert(self.data_channel.transfer_size() as f64)
        );
    }

    /// Receives an ACK packet from the server
    /// validates the block number then sends
    /// the next data block.
    fn on_ack(&mut self, ap: AckPacket) {
        let ap_blk = ap.blk();
        let packet = match self.data_channel.receive_ack(ap) {
            Ok(data_packet) => data_packet.serialize(),
            Err(error_packet) => {
                self.set_blk_error(ap_blk);
                error_packet.serialize()
            }
        };

        self.packet_buffer.push(packet);
    }

    fn set_blk_error(&mut self, actual: u16) {
        let err = format!(
            "Invalid block number [{}] expected [{}]",
            actual,
            self.data_channel.blk() + 1
        );
        self.error = Some(err);
    }
    /// Returns true if the client entered an error
    /// state.
    fn is_err(&self) -> bool {
        self.error.is_some()
    }

    /// Returns true if the transfer
    /// process is complete.
    fn is_done(&self) -> bool {
        self.data_channel.is_done()
    }

    /// Number of bytes transferred.
    fn transferred_bytes(&self) -> usize {
        self.data_channel.transfer_size()
    }

    /// Extracts the error message from the client.
    fn get_err(self) -> String {
        self.error.unwrap()
    }

    /// Set the error state for the client.
    fn on_err(&mut self, err: ErrorPacket) {
        self.error = Some(String::from(err.err()));
    }
}

/// Entry point for TFTP client.
pub fn client_main(server_address: &str, filename: &str, upload: bool) -> std::io::Result<()> {
    // Make a UDPSocket on any port on localhost.
    let sock = UdpSocket::bind("0.0.0.0:0")?;

    let mut server_address = server_address.to_string();

    let mut client = if upload {
        println!("Uploading...");
        TFTPClient::upload(filename)
    } else {
        println!("Downloading...");
        TFTPClient::download(filename)
    };

    println!("[CLIENT_ADDRESS]: {}", sock.local_addr().unwrap());

    loop {
        let mut buf = [0; 1024];

        if client.has_packets() {
            let next_packet = &client.get_next_packet();
            sock.send_to(next_packet, server_address)?;
        }

        if client.is_err() {
            panic!(client.get_err());
        }

        if client.is_done() {
            let size = convert(client.transferred_bytes() as f64);
            println!("{} bytes transferred successfully.", size);
            return Ok(());
        }

        let (count, addr) = sock.recv_from(&mut buf)?;
        // The server opens a UDP socket for each new client.
        // that's why we need to change the address to send
        // data to, otherwise we'll get an error from the
        // server. I didn't notice that on the first time I
        // tried and was getting an error, inspecting src/dst
        // port revealed that. (and it's mentioned in the RFC)
        server_address = addr.to_string();

        let raw_packet = &buf[..count];
        client.process_packet(raw_packet);
    }
}
