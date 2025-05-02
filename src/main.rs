use std::{io::Write, net::TcpStream};
#[allow(unused_imports)]
use std::net::TcpListener;

use anyhow::Error;

fn handle_message(mut stream: TcpStream) {
    let response = b"HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response).unwrap();
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");
                handle_message(_stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
