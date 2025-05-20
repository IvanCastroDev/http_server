// Imported libraries
use std::{
    collections::HashMap, io::{
        BufRead, BufReader, Write
    }, net::TcpStream, sync::Mutex
};
use std::net::TcpListener;

#[allow(unused_imports)]
use anyhow::Error;
use once_cell::sync::Lazy;

// Structs and types
type FnRoute = fn(&Request) -> String;

#[derive(Hash, Eq, PartialEq, Debug)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH
}

#[derive(Default, Debug)]
struct RouteNode {
    childs: HashMap<String, RouteNode>,
    din_child: Option<Box<RouteNode>>,
    handler: Option<FnRoute>,
    param_name: Option<String>
}

#[derive(Default)]
struct Router {
    routes: HashMap<Method, RouteNode>
}

impl Router {
    fn new() -> Self {
        Router {
            routes: HashMap::default(),
        }
    }

    fn post(&mut self, route: &str, handler: FnRoute) {
        self.add_route(Method::POST, route, handler);
    }

    fn add_route(&mut self, method: Method, route: &str, handler: FnRoute) {
        // Extramos el nodo inicial utlizando el metodo (post, get, etc) como key o llave, si el nodo no existe, insertamos uno de defecto
        let mut node = self.routes.entry(method).or_insert_with(RouteNode::default);
        let segments: Vec<String> = route.trim_matches('/').split('/').map(|s| s.to_string()).collect();

        for segment in segments {
            if segment.starts_with(":") {
                node = node.din_child.get_or_insert_with(|| Box::new(RouteNode::default()));
                node.param_name = Some(segment[1..].to_string());
            } else {
                node = node.childs.entry(segment).or_insert_with(RouteNode::default);
            }
        }
        node.handler = Some(handler);
    }

    fn exec_handler(&mut self, method: Method, route: &str, request: &Request) -> String {
        let mut res = String::new();
        let mut node = self.routes.entry(method).or_insert_with(RouteNode::default);
        let segments: Vec<String> = route.trim_matches('/').split('/').map(|s| s.to_string()).collect();
        let mut params: HashMap<String, String> = HashMap::default();

        for segment in segments {
            match node.childs.get_mut(&segment) {
                Some(child) => {
                    node = child;
                },
                None => {
                    node = node.din_child.get_or_insert_with(|| Box::new(RouteNode::default()));
                    if let Some(ref param_name) = node.param_name {
                        params.insert(param_name.clone(), segment);
                    }
                }
            }
        }

        match node.handler {
            Some(handler) => {
                res.push_str(&handler(request))
            },
            None => {
                res.push_str("404 Not Found\r\n\r\n")
            }
        }

        res
    }

    fn handle_request(&mut self, stream: TcpStream) {
        let mut request = Request::new(stream);

        let method = match self.parse_method(&request.method) {
            Ok(method) => method,
            Err(error) => {
                eprintln!("Error parsing method: {}", error);
                // Optionally, you could write an error response to the stream here
                return;
            }
        };

        println!("Request data\nmethod: {}\nroute: {}\nheaders: {:?}", request.method, request.route, request.headers);
        self.exec_handler(method, &request.route, &request);
    }

    fn parse_method(&mut self, method: &str) -> Result<Method, Error> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            _ => Err(anyhow::anyhow!("Unsupported HTTP method")),
        }
    } 
}

#[derive(Debug)]
struct Request {
    method: String,
    route: String,
    headers: Vec<String>,
    stream: TcpStream,
    params: Option<HashMap<&'static str, String>>
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
            stream: s,
            params: Some(HashMap::default())
        }
    }
}

fn exec_route_function(name: &str, request: &mut Request) {
    let mut response = String::from("HTTP/1.1 ");


    request.stream.write(response.as_bytes()).unwrap();
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

    let mut router = Router::new();

    router.post("/echo/:message", echo);
    router.post("/test/:message", echo);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");
                router.handle_request(_stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
