extern crate pretty_bytes;

use std::mem;
use std::net::UdpSocket;
use std::process::exit;

use pretty_bytes::converter::convert;

use crate::tftp::shared::{data_channel::{DataChannel, DataChannelMode}, err_packet::ErrorPacket, request_packet::{ReadRequestPacket, WriteRequestPacket}, Serializable, STRIDE_SIZE, TFTPPacket};
use crate::tftp::shared::data_channel::DataChannelOwner;

struct TFTPClient {
    packet_buffer: Option<Vec<u8>>,
    data_channel: DataChannel,
    error: Option<String>,
}

impl TFTPClient {
    /// Constructs a new TFTPClient.
    fn new(file_name: &str, mode: DataChannelMode) -> Self {
        let data_channel = DataChannel::new(file_name, mode, DataChannelOwner::Client);

        let data_channel = match data_channel {
            Ok(channel) => channel,
            Err(e) => {
                eprintln!("[ERROR] {}", e.err());
                exit(-2)
            }
        };

        // Keep the information we need to know
        // in the object and initialize them
        // to some default values.
        TFTPClient {
            packet_buffer: None,
            data_channel,
            error: None,
        }
    }

    /// Places a RRQ in the packet buffer to be sent to the server.
    pub fn download(file_name: &str) -> TFTPClient {
        let mut client = TFTPClient::new(file_name, DataChannelMode::Rx);

        let rrq = Box::new(ReadRequestPacket::new(file_name, "octet"));
        client.packet_buffer = Some(rrq.serialize());
        client
    }

    /// Places a WRQ in the packet buffer to be sent
    /// to the server, then opens the file to be read.
    pub fn upload(file_name: &str) -> TFTPClient {
        let mut client = TFTPClient::new(file_name, DataChannelMode::Tx);

        let wrq = Box::new(WriteRequestPacket::new(file_name, "octet"));
        client.packet_buffer = Some(wrq.serialize());
        client
    }

    /// Returns the first packet in the packet
    /// buffer to be sent to the server.
    pub fn get_next_packet(&mut self) -> Vec<u8> {
        let packet_at_hand = self.data_channel.packet_at_hand();
        if packet_at_hand.is_none() {
            // RRQ / WRQ are managed here.
            return mem::replace(&mut self.packet_buffer, None).unwrap();
        }

        packet_at_hand.unwrap()
    }

    /// Tells whether the client's packet buffer
    /// has any pending packets to be sent.
    pub fn is_done(&self) -> bool {
        self.data_channel.is_done()
    }

    /// Facade to client logic, parses the given buffer to a TFTP packet
    /// then acts accordingly.
    pub fn process_packet(&mut self, buf: &[u8]) {
        let packet = crate::tftp::shared::parse_udp_packet(&buf);
        println!("PACKET: {:?}", packet);
        match packet {
            TFTPPacket::DATA(data) => {
                self.data_channel.on_data(data);
                println!(
                    "Received [{}]",
                    convert(self.data_channel.transfer_size() as f64)
                );
            }
            TFTPPacket::ACK(ack) => {
                self.data_channel.on_ack(ack);
            }
            TFTPPacket::ERR(err) => self.on_err(err),
            t => panic!(format!("Unexpected packet type: [{:?}]", t)),
        };
    }

    /// Returns true if the client entered an error
    /// state.
    fn is_err(&self) -> bool {
        self.error.is_some()
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

fn check_done(client: &TFTPClient) {
    if client.is_done() {
        let size = convert(client.transferred_bytes() as f64);
        println!("{} bytes transferred successfully.", size);
        exit(0);
    }
}

/// Entry point for TFTP client.
pub fn client_main(server_address: &str, filename: &str, upload: bool) -> std::io::Result<()> {
    // Make a UDPSocket on any port on localhost.
    let sock = UdpSocket::bind("0.0.0.0:58955")?;

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

        if client.is_err() {
            eprintln!("[ERROR] {}", client.get_err());
            exit(-3);
        }

        let next_packet = &client.get_next_packet();

        sock.send_to(next_packet, server_address)?;
        println!("[OUT]");

        check_done(&client);    // Download ends here, when sending the last ACK.
        let (count, addr) = sock.recv_from(&mut buf)?;
        // The server opens a UDP socket for each new client.
        // that's why we need to change the address to send
        // data to, otherwise we'll get an error from the
        // server. I didn't notice that on the first time I
        // tried and was getting an error, inspecting src/dst
        // port revealed that. (and it's mentioned in the RFC)
        server_address = addr.to_string();

        let raw_packet = &buf[..count];
        println!("\n[IN]");
        client.process_packet(raw_packet);
    }
}
