extern crate pretty_bytes;

use std::net::{SocketAddr, UdpSocket};

use async_std::task as asyncstd_task;
use pretty_bytes::converter::convert;

use crate::tftp::shared::{parse_udp_packet, Serializable, TFTPPacket};
use crate::tftp::shared::data_channel::{DataChannel, DataChannelMode, DataChannelOwner};
use crate::tftp::shared::err_packet::{ErrorPacket, TFTPError};
use crate::tftp::shared::request_packet::{ReadRequestPacket, Request, WriteRequestPacket};

/// A TFTP server that supports a single client.
struct TFTPServer {
    data_channel: DataChannel
}

impl TFTPServer {
    pub fn new(rq_packet: &[u8]) -> Result<Self, ErrorPacket> {
        match parse_udp_packet(rq_packet) {
            TFTPPacket::RRQ(rrq) => TFTPServer::init_rrq_response(rrq),
            TFTPPacket::WRQ(wrq) => TFTPServer::init_wrq_response(wrq),
            _ => panic!(),
        }
    }


    pub fn is_err(&self) -> bool {
        self.data_channel.is_err()
    }
    pub fn err(self) -> String {
        self.data_channel.err()
    }
    pub fn blk(&self) -> u16 {
        self.data_channel.blk()
    }

    pub fn run(&mut self, raw_packet: &[u8]) {
        let p = parse_udp_packet(raw_packet);
        match p {
            TFTPPacket::ERR(ep) => panic!("Terminating client: {}", ep.err()),
            TFTPPacket::ACK(ack) => self.data_channel.on_ack(ack),
            TFTPPacket::DATA(data) => self.data_channel.on_data(data),
            p => panic!("Illegal packet {:?}", p),
        };
    }

    fn init_rrq_response(rrq: ReadRequestPacket) -> Result<TFTPServer, ErrorPacket> {
        DataChannel::new(rrq.filename(), DataChannelMode::Tx, DataChannelOwner::Server)
            .and_then(|data_channel| {
                let server = TFTPServer { data_channel };
                Ok(server)
            })
    }

    fn init_wrq_response(wrq: WriteRequestPacket) -> Result<TFTPServer, ErrorPacket> {
        DataChannel::new(wrq.filename(), DataChannelMode::Rx, DataChannelOwner::Server)
            .and_then(|data_channel| {
                let server = TFTPServer { data_channel };
                Ok(server)
            })
    }

    fn get_next_packet(&mut self) -> Vec<u8> {
        self.data_channel.packet_at_hand().unwrap()
    }

    fn done(&self) -> bool {
        self.data_channel.is_done()
    }
}

fn handle_client(socket: UdpSocket, mut server: TFTPServer, client_addr: SocketAddr) {
    // asyncstd_task::spawn(async move {
    loop {
        if server.is_err() {
            eprintln!("Fatal error: {}", server.err());
            panic!();
        }

        if server.done() {
            break;  // If we sent the last data packet in the previous loop
        }

        let p = server.get_next_packet();
        println!("Sending #{} [{}]", server.blk(), convert(p.len() as f64));
        socket.send_to(&p, client_addr).unwrap();

        if server.done() {
            break;  // If we've just sent the last ack
        }

        let mut buf = [0 as u8; 1024];
        let (count, addr) = socket
            .recv_from(&mut buf)
            .expect("Failed to read socket fd");
        let raw_msg = &buf[..count];

        if addr != client_addr {
            let error_packet = ErrorPacket::new(TFTPError::UnknownTID);
            socket.send_to(&error_packet.serialize(), addr).unwrap();
        }

        server.run(raw_msg);
    }
    // });
}

pub fn handle_new_client(client_addr: SocketAddr, rq_packet: &[u8]) {
    println!("New connection: {}", client_addr);
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket");

    match TFTPServer::new(rq_packet) {
        Ok(server) => {
            handle_client(socket, server, client_addr);
        }
        Err(error_packet) => {
            eprintln!("Terminating client [{}]", error_packet.err());
            socket
                .send_to(&error_packet.serialize(), client_addr)
                .unwrap();
            drop(socket);
        }
    }
}

pub fn server_main(address: &str, port: u16) {
    let addr = format!("{}:{}", address, port);
    let sock = UdpSocket::bind(addr).expect("Failed to bind UDP socket");
    println!("[SERVER_ADDRESS]: {}", sock.local_addr().unwrap());

    let f = async {
        loop {
            let mut buf = [0; 1024];
            let (count, addr) = sock.recv_from(&mut buf).unwrap();

            let raw_packet = &buf[..count];
            match parse_udp_packet(raw_packet) {
                TFTPPacket::RRQ(_) | TFTPPacket::WRQ(_) => {
                    handle_new_client(addr, raw_packet);
                }
                _ => {
                    let err = ErrorPacket::new(TFTPError::IllegalOperation);
                    sock.send_to(&err.serialize(), addr).unwrap();
                }
            }
        }
    };
    asyncstd_task::block_on(f);
}
