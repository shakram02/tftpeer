use crate::tftp::common::request_packet::{ReadRequestPacket, WriteRequestPacket};
use crate::tftp::common::Serializable;

struct TFTPClient {
    server_address: String,
    packet_buffer: Vec<u8>,
}

impl TFTPClient {
    fn new(file_name: String, server_address: String) -> Self {
        TFTPClient {
            server_address,
            packet_buffer: Vec::new(),
        }
    }

    pub fn download(&mut self, file_name: &str) {
        let rrq = ReadRequestPacket::new(file_name, "octet");

        self.packet_buffer = rrq.serialize()
    }
    pub fn upload(&mut self, file_name: &str) {
        let wrq = WriteRequestPacket::new(file_name,
                                          "octet");

        self.packet_buffer = wrq.serialize()
    }

    fn process_packet(buf: Vec<u8>) {}

    fn get_buffer(mut self) -> Vec<u8> {
        let buf = self.packet_buffer;
        self.packet_buffer = Vec::new();

        buf
    }
}