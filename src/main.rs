// libraries
use std::{
    collections::{
        HashMap,
        VecDeque
    }, io::{
        BufRead,
        BufReader,
        Read, 
        Write
    }, 
    net::TcpStream, 
    path::Path, 
    str,
    thread,
    time::Duration
};
use std::net::TcpListener;

#[allow(unused_imports)]
use anyhow::Error;

// Structs and types
type FnRoute = fn(&Request) -> String;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH
}

#[derive(Default, Debug, Clone)]
struct RouteNode {
    childs: HashMap<String, RouteNode>,
    din_child: Option<Box<RouteNode>>,
    handler: Option<FnRoute>,
    param_name: Option<String>
}

#[derive(Default, Clone)]
struct Router {
    routes: HashMap<Method, RouteNode>
}

impl Router {
    fn new() -> Self {
        Router {
            routes: HashMap::default(),
        }
    }

    #[allow(dead_code)]
    fn post(&mut self, route: &str, handler: FnRoute) {
        self.add_route(Method::POST, route, handler);
    }

    #[allow(dead_code)]
    fn get(&mut self, route: &str, handler: FnRoute) {
        self.add_route(Method::GET, route, handler);
    }

    #[allow(dead_code)]
    fn put(&mut self, route: &str, handler: FnRoute) {
        self.add_route(Method::PUT, route, handler);
    }

    #[allow(dead_code)]
    fn delete(&mut self, route: &str, handler: FnRoute) {
        self.add_route(Method::DELETE, route, handler);
    }

    fn add_route(&mut self, method: Method, route: &str, handler: FnRoute) {
        // First, extract the initial node using the method (post, get, etc) as key, if the node does not exist, insert one
        // The value of the attribute "routes" is a hashmap that uses an enum of methods as a keys
        // {
        //   GET: {
        //     RouteNode({
        //        ...
        //     })
        //   },
        //   POST: {
        //     RouteNode({
        //        ...
        //     })
        //   }      
        // }
        let mut node = self.routes.entry(method).or_insert_with(RouteNode::default);
        
        // separate the route into segments, taking as a separator the character "/"
        let segments: Vec<String> = route.trim_matches('/').split('/').map(|s| s.to_string()).collect();

        // Iterate between each string within the segments vector and check if its value is dynamic or static.
        for segment in segments {
            if segment.starts_with(":") {
                // If the string begins with ":" It means that its value is dynamic, therefore, create a dynamic child inside the node
                node = node.din_child.get_or_insert_with(|| Box::new(RouteNode::default()));

                // keep the name of the parameter within the param_name attribute of the node
                node.param_name = Some(segment[1..].to_string()); // 1 .. to ignore the first character (":")
            } else {
                // On the contrary, by not starting the segment with ":" it means that its value is static, therefore, 
                // can access the children of the node with the value of the segment as Key or create a new one if doesn't exist
                node = node.childs.entry(segment).or_insert_with(RouteNode::default);
            }
        }

        // Once finish iterating between the segments, it means that we are at the end of our route tree, knowing this, we can inject the Handler function in the node
        node.handler = Some(handler);
    }

    fn exec_handler(&mut self, method: Method, request: &mut Request) -> String {
        let mut res = String::new();
        let mut node = self.routes
                                            .entry(method)
                                            .or_insert_with(RouteNode::default);

        let segments: Vec<String> = request.route
                                    .trim_matches('/')
                                    .split('/')
                                    .map(|s| s.to_string())
                                    .collect();
                                
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

        request.params = params;

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
        let mut response = String::from("HTTP/1.1 ");


        let method = match self.parse_method(&request.method) {
            Ok(method) => method,
            Err(error) => {
                eprintln!("Error parsing method: {}", error);
                response.push_str("405 Method Not Allowed\r\nAllow: GET, POST, PUT, DELETE, PATCH\r\n\r\n");
                request.stream.write(response.as_bytes()).unwrap();
                return;
            }
        };

        response.push_str(&self.exec_handler(method, &mut request));

        request.stream.write(response.as_bytes()).unwrap();
    }

    fn parse_method(&mut self, method: &str) -> Result<Method, Error> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            _ => Err(anyhow::anyhow!("Unsupported HTTP method {}", method.to_uppercase())),
        }
    } 
}

#[derive(Debug)]
#[allow(dead_code)]
struct Request {
    method: String,
    route: String,
    headers: HashMap<String, String>,
    stream: TcpStream,
    params: HashMap<String, String>,
    body: HashMap<String, String>,
    files: Vec<(HashMap<String, String>, Vec<u8>)>
}

impl Request  {
    fn new (mut s: TcpStream) -> Self {
        // use bufReader to read the stream bytes in fragments (8kib).
        let mut reader = BufReader::new(&mut s);
        let mut request_data = String::new();

        loop {
            // String as a bufer to translate the bytes read in a string
            let mut str_buf = String::new();

            // read and count the amount of bytes in the stream before a crlf that marks the end of a line
            let bytes_read = reader.read_line(&mut str_buf).unwrap();

            // If the bytes reads are equal to zero it is because the connection with the client is closed
            if bytes_read == 0 {
                break;
            };

            // End of the headers and request line 
            if str_buf == "\r\n" {
                break;
            };
            
            // add the new line to the string that stores all the data from the request
            request_data.push_str(&str_buf);
        }

        // separate the data to divide them in [request line, headers]
        let mut request_data: VecDeque<String> = request_data // VecDeque to extract elements at the beginning or end of the vector more simply
            .split("\r\n")
            .map(|s| s.to_string())
            .collect();

        // extract the data from the request line to be able to throw the method and the destination route
        let start_line = request_data.pop_front().unwrap_or_else(|| "start line".to_string());
        let start_line_parts: Vec<&str> = start_line.split(" ").collect(); // [Method, dest_route]

        let headers_map: HashMap<String, String> = request_data.
            into_iter().
            filter_map(|l| {
                let mut line_split = l.splitn(2, ":"); // separate the string using the value of ":" as a separator, we limit the separation of the string so that only 2 parts return
                let key = line_split.next()?.trim().to_string(); // The first part is taken as the hashmap key
                let value = line_split.next()?.trim().to_string(); // The second part is taken as the value of the previously established key

                Some((key, value))
            })
            .collect();

        let content_bytes =  headers_map
            .get("Content-Length")
            .unwrap_or(&String::from("0"))
            .parse::<usize>()
            .unwrap_or(0);

        let default_boudnary = String::from("boundary=------");

        let boundary = headers_map
            .get("Content-Type")
            .unwrap_or(&default_boudnary)
            .split("boundary=")
            .nth(1)
            .unwrap_or("--------");

        let boundary_string = format!("--{}", boundary);
        let boundary_bytes = boundary_string.as_bytes();

        let mut body_buf = vec![0u8; content_bytes];

        reader.read_exact(&mut body_buf).unwrap();
        
        let mut req = Self {
            method: start_line_parts[0].to_string(),
            route: start_line_parts[1].to_string(),
            headers: headers_map,
            params: HashMap::default(),
            stream: s,
            body: HashMap::default(),
            files: Vec::default()
        };

        let parts = req.split_multipart(&body_buf, boundary_bytes);

        for part in parts {
            let parsed_part = req.parse_part(part);

            req.parse_body(parsed_part);
        }

        req
    }

    fn parse_body(&mut self, part: (HashMap<String, String>, &[u8])) {
        if let Some(_filename) = part.0.get("filename") {
            // leave the bites of the file intact to be processed manually by the user
            self.files.push((part.0, part.1.to_vec()));
        } else {
            let key = part.0.
                                get("name")
                                .unwrap_or(&String::from("key"))
                                .to_string();
            // convert the bites into text, which will be used as the hashmap value
            let value = std::str::from_utf8(part.1)
                                    .unwrap()
                                    .trim()
                                    .to_string();

            self.body.insert(key, value);
        }
    }

    fn parse_part<'a>(&mut self, part: &'a[u8]) -> (HashMap<String, String>, &'a[u8]) {
        let crlf = b"\r\n\r\n";

        // find the end of the headers and start of the body of the request part based on a CRLF
            // Eg:
            //
            // |----------------------------------------------------------------|
            // |                         Headers                                |
            // |----------------------------------------------------------------|
            // ---- \r\n\r\n (CRLF)
            // |----------------------------------------------------------------|
            // |                         Body                                   |
            // |----------------------------------------------------------------|

        if let Some(index) = part.windows(4).position(|w| w == crlf) {
            let headers_bytes = &part[..index];
            let data = &part[index + 4..]; // Skip CRLF 4 bites

            let mut headers: HashMap<String, String> = HashMap::new();

            for line in headers_bytes.split(|&b| b == b'\n') { // \n is equivalent to having finished a header line, so bytes are divided based on this element
                if let Some(split_index) = line.windows(2).position(|b| b == b": ") { // separate "key: value", Eg. Content-Type: application/json
                    let key = std::str::from_utf8(&line[..split_index])
                                        .unwrap_or("")
                                        .trim()
                                        .to_string();
                                    
                    let values = std::str::from_utf8(&line[split_index + 2..])
                                            .unwrap_or("")
                                            .trim()
                                            .to_string();

                    let values_iter = values.split("; ").skip(1); // skip the first element, which is equal to "form-data;", we look for only values ​​that are equal to "value=value;" Eg. Name="Test"

                    for value in values_iter {
                        if let Some((k, v)) = value.split_once("=") {
                            headers.insert(
                                k.replace("\"", ""), 
                                v.replace("\"", "")
                            );
                        }
                    }

                    headers.insert(
                        key, 
                        values
                            .split_terminator("; ")
                            .nth(0).unwrap_or("")
                            .to_string());
                }
            }

            return (headers, data)
        }

        (HashMap::new(), &[])
    }

    fn split_multipart<'a>(&mut self, body: &'a [u8], boundary: &[u8]) -> Vec<&'a [u8]> {
        let mut parts: Vec<&'a [u8]> = Vec::new();
        let mut pos = 0;

        // search in the body vector the bytes that are identical to the Boundary bytes
        // When find an identical bytes chain, ignore them
        // only extracted those bytes that are between each Boundary
            // Eg:
            //
            // ---- first boundary finded {This part is ignored} - start variable
            // |----------------------------------------------------------------|
            // |                         Content                                | - This part is taken and pushed in parts vector
            // |----------------------------------------------------------------|
            // ---- Next boundary - end variable
            // |----------------------------------------------------------------|
            // |                         Content                                | - Taked and pushed again
            // |----------------------------------------------------------------|
            // ---- Next or final boundary

        while let Some(start) = self.find_boundary(body, boundary, pos) {
            // Start + Boundary - Because the Start value is equal to the initial index where the Boudnary was found in the body vec, therefore, the bytes that are Boundary himself are ignored
            if let Some(end) = self.find_boundary(body, boundary, start + boundary.len()) {
                let part = &body[start + boundary.len() + 2 ..end]; // + 2 to ignore the bytes of \r\n and end the sub-slice in the next boundary finded
                parts.push(part);
                pos = end;
            } else {
                break;
            }
        };

        parts
    }
    
    fn find_boundary(&mut self, body: &[u8], boundary: &[u8], start: usize) -> Option<usize> {
        // split the Body bytes in blocks of the same size as the Boundary and iterate between each block
        // Starting from the index indicating the start param (the value of this param will be equal to 0 or to the last index in which a Boundary was found so as not to repeat bytes already compared)
        // If the block contains the same bites as the Boundary, the index of the first elemento in the block is extracted and returned
        body[start..]
            .windows(boundary.len()) // break the sub-slice in blocks of "n" size (in this case, blocks of the same size as the Boundary)
            .position(|window| window == boundary) // searching the window that contains the same bytes as the Boundary
            .map(|index| index + start) // index + start - Because the value of the index is relative to the sub-slice created by [start ..], we need to know the absolute value of the index in the body

    }
}

//Functions for routes
fn echo(request: &Request) -> String {
    if let Some(message) = request.params.get("message") {
        String::from(format!("200 Ok\r\nContent-Type:text/plain\r\nContent-Length: {}\r\n\r\n{}", message.len(), message))
    } else {
        let message: String = String::from("Invalid Message");
        String::from(format!("400 Bad Request\r\nContent-Type:text/plain\r\nContent-Length: {}\r\n\r\n{}", message.len(), message))
    }
}

fn test_post_files(request: &Request) -> String {
    if !Path::new("./uploads").is_dir() {
        std::fs::create_dir("./uploads").unwrap();
    }

    for file in request.files.iter() {
        let file_path = format!("./uploads/{}", file.0.get("filename").unwrap_or(&"test.txt".to_string()));

        std::fs::write(
            file_path, 
            &file.1
        ).unwrap();
    }
    
    String::from(format!("200 Ok\r\nContent-Type:text/plain\r\n\r\nDone"))
}

fn slow_request(request: &Request) -> String {
    let duration = request.params
            .get("duration")
            .unwrap_or(&"5".to_string())
            .parse::<u64>()
            .unwrap_or(5);

    println!("Sleeping request for {} seconds", duration);
    
    // simulating a slow request
    thread::sleep(Duration::from_secs(duration));

    String::from(format!("200 Ok\r\nContent-Type:text/plain\r\n\r\nDone"))
}

fn main() {
    let mut router = Router::new();

    router.get("/", |_r| String::from("200 Ok\r\nContent-Type:text/plain\r\n\r\nDone"));
    router.get("/sleep/:duration", slow_request);
    router.get("/echo/:message", echo);
    router.get("/test/:message", echo);
    router.post("/post_test", test_post_files);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");
                let mut router = router.clone();
                thread::spawn(move || {
                    router.handle_request(_stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}