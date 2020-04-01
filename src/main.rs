use clap::Clap;

use crate::tftp::client::client_main;
use crate::tftp::server::server_main;

mod tftp;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "shakram02")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    /// act as a TFTP client.
    #[clap(name = "client")]
    Client(ClientOperations),
    /// act as a TFTP server.
    #[clap(name = "server")]
    Server(ServerArgs),
}

#[derive(Clap, Debug)]
struct ServerArgs {
    /// IP for the server to use.
    #[clap(short = "a", long = "address", default_value = "127.0.0.1")]
    address: String,
    /// UDP port that the server will listen on.
    #[clap(short = "p", long = "port", default_value = "69")]
    port: u16,
}

/// A subcommand for controlling testing
#[derive(Clap, Debug)]
struct ClientOperations {
    /// name of the file to be downloaded.
    filename: String,
    /// If specified tftpeer will attempt to upload the input file
    #[clap(short = "u", long = "upload")]
    upload: bool,
    /// Server bind address
    #[clap(short = "a", long = "address", default_value = "127.0.0.1")]
    address: String,
    /// Server bind port
    #[clap(short = "p", long = "port", default_value = "69")]
    port: u16,
}

fn main() {
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Client(client_args) => {
            let addr = format!("{}:{}", client_args.address, client_args.port);
            if client_args.upload {
                println!(
                    "[UPLOAD] FILE: ({}) TO SERVER: {}",
                    client_args.filename, addr
                );
            } else {
                println!(
                    "[DOWNLOAD] FILE: ({}) SERVER: {}",
                    client_args.filename, addr
                );
            }

            client_main(&addr, &client_args.filename, client_args.upload).unwrap();
        }
        SubCommand::Server(server_args) => {
            server_main(&server_args.address, server_args.port);
        }
    };
}
