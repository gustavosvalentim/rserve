use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, prelude::*};
use std::net::TcpStream;

pub fn handle_http_request(mut stream: &TcpStream) -> HttpResponse {
    let mut request = HttpRequest::parse(stream);
    let mut response = HttpResponse::new();

    println!("Method: {:?} - Path: {:?}", request.method, request.path);

    if request.path == "/" {
        request.path = String::from("/index.html");
    }

    let filepath = format!("{}{}", std::env::current_dir().unwrap().to_str().unwrap(), request.path);
    let file_ext = filepath.split('.').last().unwrap();
    let content_type = match file_ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "jpeg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    };

    let content = match fs::read_to_string(filepath) {
        Ok(content) => content,
        Err(_) => {
            response.status_code = 404;
            response.status_text = String::from("NOT FOUND");
            String::new()
        },
    };
    let length = content.len();

    response.content = content;
    response.headers.insert(String::from("Content-Length"), length.to_string());
    response.headers.insert(String::from("Content-Type"), content_type.to_string());

    stream.write_all(response.to_text().as_bytes()).unwrap();
    
    response
}

pub struct HttpRequest {
    pub method: String,
    pub path: String,
}

impl HttpRequest {
    pub fn parse(stream: &TcpStream) -> Self {
        let buf_reader = BufReader::new(stream);
        let request_lines: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();
        
        let status_line: Vec<_> = request_lines[0].split(' ').collect();
        
        Self{
            method: status_line[0].to_string(),
            path: status_line[1].to_string(),
        }
    }
}

pub struct HttpResponse {
    pub status_code: i32,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub content: String,
}

impl HttpResponse {
    pub fn new() -> Self {
        Self{
            status_code: 200,
            status_text: String::from("OK"),
            headers: HashMap::new(),
            content: String::new(),
        }
    }
    
    pub fn to_text(&self) -> String {
        let mut text = String::new();

        text.push_str(format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_code).as_str());
        
        for (key, value) in &self.headers {
            text.push_str(format!("{}: {}\r\n", key, value).as_str());
        }

        text.push_str("\r\n");
        text.push_str(&self.content);
        text.push_str("\r\n\r\n");
        
        text
    }
}