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

struct TFTPClient {
    packet_buffer: Vec<Vec<u8>>,
    blk: u16,
    fd: Option<File>,
    file_name: String,
    done: bool,
    error: Option<String>,
    bytes: usize,
}

impl TFTPClient {
    /// Constructs a new TFTPClient.
    fn new() -> Self {
        // Keep the information we need to know
        // in the object and initialize them
        // to some default values.
        TFTPClient {
            packet_buffer: Vec::new(),
            blk: 0,
            bytes: 0,
            fd: Option::None,
            file_name: String::new(),
            done: false,
            error: None,
        }
    }

    /// Places a RRQ in the packet buffer to be sent to the server.
    pub fn download(&mut self, file_name: &str) {
        let rrq = ReadRequestPacket::new(file_name, "octet");
        self.file_name = String::from(file_name);
        self.packet_buffer.push(rrq.serialize());
    }

    /// Places a WRQ in the packet buffer to be sent
    /// to the server, then opens the file to be read.
    pub fn upload(&mut self, file_name: &str) {
        let wrq = WriteRequestPacket::new(file_name, "octet");
        self.packet_buffer.push(wrq.serialize());

        let fd = File::open(Path::new(file_name)).unwrap();
        self.fd = Some(fd);
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
        if self.blk + 1 != dp.blk() {
            let err = format!("Invalid block number [{}] expected [{}]", dp.blk(), self.blk + 1);
            self.do_err(TFTPError::IllegalOperation);
            self.error = Some(err);
            return;
        }

        self.blk = dp.blk();

        // First data block, open a new file.
        if self.blk == 1 {
            let fd = File::create(Path::new(&self.file_name)).unwrap();
            self.fd = Some(fd);
        }

        let mut fd = self.fd.as_ref().unwrap();
        let data = &dp.data();

        self.bytes += data.len();
        fd.write_all(data).unwrap();

        // Final block, close file.
        if data.len() != STRIDE_SIZE {
            self.done = true;
            self.fd = None;
        }

        println!("Received [{}]", convert(data.len() as f64));
        self.do_ack();
    }

    /// Receives a data block from the server,
    /// writes it to the file then sends an
    /// ACK packet to the server.
    fn do_data(&mut self) {
        let mut buf = [0; STRIDE_SIZE];
        self.blk += 1;
        let mut fd = self.fd.as_ref().unwrap();
        let bytes_read = fd.read(&mut buf).unwrap();

        if bytes_read < STRIDE_SIZE {
            self.done = true;
            self.fd = None;
        }

        self.bytes += bytes_read;
        let data = Vec::from(&buf[0..bytes_read]);
        let dp = DataPacket::new(self.blk, data);

        println!("Sent [{}]", convert(bytes_read as f64));
        self.packet_buffer.push(dp.serialize());
    }

    /// Receives an ACK packet from the server
    /// validates the block number then sends
    /// the next data block.
    fn on_ack(&mut self, ap: AckPacket) {
        if self.blk != ap.blk() {
            let err = format!("Invalid block number [{}], expected [{}]", ap.blk(), self.blk + 1);
            self.do_err(TFTPError::IllegalOperation);
            self.error = Some(err);
            return;
        }

        self.do_data();
    }

    /// Send an ACK packet to the server.
    fn do_ack(&mut self) {
        let ack = AckPacket::new(self.blk);
        self.packet_buffer.push(ack.serialize());
    }

    /// Set the error state for the client.
    fn on_err(&mut self, err: ErrorPacket) {
        self.error = Some(String::from(err.err()));
    }

    /// Send an error packet to the server.
    fn do_err(&mut self, error: TFTPError) {
        let err = ErrorPacket::new(error);
        self.packet_buffer.push(err.serialize());
    }

    /// Returns true if the client entered an error
    /// state.
    fn is_err(&self) -> bool {
        self.error.is_some()
    }

    /// Returns true if the transfer
    /// process is complete.
    fn is_done(&self) -> bool {
        self.done
    }

    /// Number of bytes transferred.
    fn transferred_bytes(&self) -> usize {
        self.bytes
    }

    /// Extracts the error message from the client.
    fn get_err(self) -> String {
        self.error.unwrap()
    }
}

/// Entry point for TFTP client.
pub fn client_main(server_address: &str, filename: &str, upload: bool) -> std::io::Result<()> {
    // Make a UDPSocket on any port on localhost.
    let sock = UdpSocket::bind("0.0.0.0:0")?;

    let mut server_address = server_address.to_string();
    let mut client = TFTPClient::new();

    if upload {
        client.upload(filename);
        println!("Uploading...");
    } else {
        client.download(filename);
        println!("Downloading...");
    }

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

    Ok(())
}
