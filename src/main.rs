use std::{
    io::{
        BufRead, BufReader, Write
    }, 
    net::TcpStream,
    collections::HashMap
};
use std::net::TcpListener;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[allow(unused_imports)]
use anyhow::Error;

#[derive(Debug)]
struct Request {
    method: String,
    route: String,
    headers: Vec<String>
}

fn echo_request(stream: &TcpStream) {

}

static ROUTES: Lazy<Mutex<HashMap<&'static str, Box<dyn Fn(&TcpStream) + Send + Sync>>>> = Lazy::new(|| {
    #[warn(unused_mut)]
    let mut routes: HashMap<&'static str, Box<dyn Fn(&TcpStream) + Send + Sync>> = HashMap::new();

    Mutex::new(routes)
});

fn handle_request(mut stream: TcpStream) {
    let request = read_stream(&stream);
    println!("Request data\nmethod: {}\nroute: {}\nheaders: {:?}", request.method, request.route, request.headers);

    let mut response = String::from("HTTP/1.1 ");
    
    match request.route {
        route if route == "/" => &response.push_str("200 OK\r\n\r\n"),
        _ => &response.push_str("404 Not Found\r\n\r\n"),
    };

    stream.write(response.as_bytes()).unwrap();
}

/// Method to translate the request data
fn read_stream(stream: &TcpStream) -> Request {
    // We use bufReader to read the stream bytes in fragments (8kib).
    let mut reader = BufReader::new(stream);
    let mut request_data = String::new();

    loop {
        // String as a bufer to translate the bytes read in a string
        let mut str_buf = String::new();

        // We read and count the amount of bytes in the stream before a crlf that marks the end of a line
        let bytes_read = reader.read_line(&mut str_buf).unwrap();

        // If the reads are equal to zero it is because the connection with the client is closed
        if bytes_read == 0 {
            break;
        };

        // End of the headers and request line 
        if str_buf == "\r\n" {
            break;
        };
        
        // We add the new line to the string that stores all the data from the request
        request_data.push_str(&str_buf);
    }

    // We separate the data to divide them in [request line, headers]
    let request_data: Vec<String> = request_data
        .split("\r\n")
        .map(|s| s.to_string())
        .collect();

    // We extract the data from the petition line to be able to throw the method and the destination route
    let start_line = &request_data[0];
    let start_line_parts: Vec<&str> = start_line.split(" ").collect();


    // We return a struct with the formatted request data
    Request {
        method: start_line_parts[0].to_string(),
        route: start_line_parts[1].to_string(),
        headers: request_data
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");
                handle_request(_stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
