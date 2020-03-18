use std::fs::File;
use std::io::Write;
use std::net::UdpSocket;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use crate::tftp::common::ack_packet::AckPacket;
use crate::tftp::common::data_packet::DataPacket;
use crate::tftp::common::err_packet::ErrorPacket;
use crate::tftp::common::request_packet::{ReadRequestPacket, WriteRequestPacket};
use crate::tftp::common::{parse_udp_packet, Serializable, TFTPPacket};

struct TFTPClient {
    packet_buffer: Vec<Vec<u8>>,
    blk: u16,
    fd: Option<File>,
}

impl TFTPClient {
    fn new() -> Self {
        TFTPClient {
            packet_buffer: Vec::new(),
            fd: Option::None,
            blk: 0,
        }
    }

    pub fn download(&mut self, file_name: &str) {
        let rrq = ReadRequestPacket::new(file_name, "octet");
        let fd = File::create(Path::new(file_name)).unwrap();

        self.packet_buffer.push(rrq.serialize());
        self.fd = Some(fd);
    }
    pub fn upload(&mut self, file_name: &str) {
        let wrq = WriteRequestPacket::new(file_name, "octet");
        // next ack is 0
        self.packet_buffer.push(wrq.serialize())
    }

    pub fn get_next_packet(&mut self) -> Vec<u8> {
        self.packet_buffer.remove(0)
    }

    pub fn has_packets(&self) -> bool {
        self.packet_buffer.len() != 0
    }

    pub fn process_packet(&mut self, buf: &[u8]) {
        let packet = crate::tftp::common::parse_udp_packet(&buf);
        println!(">> {}", packet.to_string());
        match packet {
            TFTPPacket::ACK(ack) => self.on_ack(ack),
            TFTPPacket::ERR(err) => self.on_err(err),
            TFTPPacket::DATA(data) => self.on_data(data),
            t => panic!(format!("Unexpected packet type: [{:?}]", t)),
        };
    }

    fn on_ack(&self, ap: AckPacket) {}

    fn on_data(&mut self, dp: DataPacket) {
        if self.blk + 1 != dp.blk() {
            panic!(format!("Bad packet number [{}]", dp.blk()));
            self.do_err("invalid block number");
        } else {
            self.blk = dp.blk();
        }

        let mut fd = self.fd.as_ref().unwrap();
        fd.write_all(&dp.data()).unwrap();
        self.do_ack();
    }

    fn on_err(&self, err: ErrorPacket) {}

    fn do_err(&self, error: &str) {
        // TODO: send error packet.
    }

    fn do_ack(&mut self) {
        let ack = AckPacket::new(self.blk);
        println!("Sending: {:?}", ack);
        self.packet_buffer.push(ack.serialize());
    }
}

pub fn client_main(server_address: &str, filename: &str) -> std::io::Result<()> {
    println!("****************************************");
    println!(
        "[SERVER_ADDRESS]: {}\n[FILE_NAME]:{}",
        server_address, filename
    );
    println!("****************************************");

    let sock = UdpSocket::bind("0.0.0.0:33797")?;
    let mut server_address = server_address.to_string();
    let mut client = TFTPClient::new();
    client.download(filename);
    // sock.send_to(&request.serialize(), server_address);

    println!("[CLIENT_ADDRESS]: {}", sock.local_addr().unwrap());
    println!("Downloading...");
    loop {
        // sleep(Duration::from_millis(800));
        println!("zzz....");

        let mut buf = [0; 1024];
        if client.has_packets() {
            sock.send_to(&client.get_next_packet(), server_address)?;
        }

        let (count, addr) = sock.recv_from(&mut buf)?;
        server_address = addr.to_string();

        let raw_packet = &buf[..count];
        println!("got {} bytes from {}", count, addr);

        client.process_packet(raw_packet);
    }

    Ok(())
}
