// Imported libraries
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

// Structs and types
type FnRoute = fn(&Request) -> String;

enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH
}

#[derive(Default)]
struct RouterNode {
    child: HashMap<&'static str, RouterNode>,
    din_child: Option<Box<RouterNode>>,
    handler: Option<FnRoute>
}

#[derive(Default)]
struct Router {
    routes: HashMap<Method, HashMap<&'static str, RouterNode>>
}

impl Router {
    fn new() -> Self {
        Router {
            routes: HashMap::default()
        }
    }

    fn post(route: &str, handler: FnRoute) {
        Self::add_route(Method::POST, route, handler);
    }

    fn add_route(method: Method, route: &str, handler: FnRoute) {
        
    }
}

#[derive(Debug)]
struct Request {
    method: String,
    route: String,
    headers: Vec<String>,
    stream: TcpStream
}

impl Request  {
    fn new (mut s: TcpStream) -> Self {
        // We use bufReader to read the stream bytes in fragments (8kib).
        let mut reader = BufReader::new(&mut s);
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
            headers: request_data,
            stream: s
        }
    }
}

// Utilities
fn parse_method(method: &str) -> Method {
    match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        _ => panic!("Unsupported HTTP method"),
    }
} 

/* fn exec_route_function(name: &str, request: &mut Request) {

    let mut response = String::from("HTTP/1.1 ");

    match routes.get(name) {
        Some(f) => {
            response.push_str(&f(request));
        },
        None => response.push_str("404 Not Found\r\n\r\n")
    };

    request.stream.write(response.as_bytes()).unwrap();
} */

fn handle_request(stream: TcpStream) {
    let mut request = Request::new(stream);
    println!("Request data\nmethod: {}\nroute: {}\nheaders: {:?}", request.method, request.route, request.headers);
    exec_route_function("/echo", &mut request);
}

//Functions for routes destinations
fn echo(request: &Request) -> String {
    let reg = regex::Regex::new(r"^/echo/(?P<message>[^/]+)$").unwrap();
    
    if let Some(caps) = reg.captures(&request.route) {
        let message = &caps["message"];

        String::from(format!("200 Ok\r\nContent-Type:text/plain\r\nContent-Length: {}\r\n\r\n{}", message.len(), message))
    } else {
        let message: String = String::from("Invalid Message");
        String::from(format!("400 Bad Request\r\nContent-Type:text/plain\r\nContent-Length: {}\r\n\r\n{}", message.len(), message))
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
