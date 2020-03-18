use crate::tftp::client::client_main;

mod tftp;

fn main() -> std::io::Result<()> {
    client_main("127.0.0.1:69", "res.txt")
}
