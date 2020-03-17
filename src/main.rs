use crate::tftp::common::{request_packet::ReadRequestPacket, Serializable};
use crate::tftp::common::request_packet::Request;

mod tftp;

fn main() {
    tftp::hello();
    let packet = ReadRequestPacket::new("a.txt", "octet");
    println!("{:X?}", packet.serialize());
    // let sock = UdpSocket::bind("127.0.0.1:0").expect("Couldn't bind to address");
    // println!("Listening on {0}...", sock.local_addr().unwrap());

    // let mut buf = [0; 4096];
    // let (number_of_bytes, src_address) =
    //     sock.recv_from(&mut buf).expect("Couldn't receive message");

    // println!("Received {0} bytes from {1}", number_of_bytes, src_address);
}
