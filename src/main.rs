use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::{prelude::*, BufReader, Error};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

pub fn handle_connection(mut stream: &TcpStream) -> HttpResponse {
    let basedir = PathBuf::from(std::env::current_dir().unwrap().to_str().unwrap());
    let mut request = HttpRequest::parse(stream);
    let mut response = HttpResponse::new();

    println!(
        "Request {} {} {}",
        request.method, request.path, request.http_version
    );

    if request.path == "/" {
        request.path = String::from("/index.html");
    }

    let path = basedir.join(request.path.strip_prefix("/").unwrap());

    if path.is_dir() {
        // TODO: handle directory here
    } else {
        let (content, file_metadata) = match read_file(&path) {
            Ok((content, file_metadata)) => (content, file_metadata),
            Err(_) => {
                response.status_code = 404;
                response.status_text = String::from("NOT FOUND");
                return response;
            }
        };

        response.content = String::from_utf8(content).unwrap();
        response.headers.insert(
            String::from("Content-Length"),
            file_metadata.size.to_string(),
        );
        response.headers.insert(
            String::from("Content-Type"),
            file_metadata.content_type.to_string(),
        );

        stream.write_all(response.to_text().as_bytes()).unwrap();
    }

    println!(
        "Response {} {} {}",
        request.http_version, response.status_code, response.status_text
    );

    response
}

fn find_mime_type(path: &PathBuf) -> &'static str {
    let extension = path.extension();
    let mut mime_type = "application/octet-stream";

    if let Some(extension) = extension {
        mime_type = match extension.to_str().unwrap_or("") {
            "css" => "text/css",
            "js" => "text/javascript",
            "jpeg" => "image/jpeg",
            "png" => "image/png",
            "svg" => "image/svg+xml",
            "wasm" => "application/wasm",
            "html" => "text/html",
            _ => mime_type,
        }
    }

    mime_type
}

fn read_file(path: &PathBuf) -> Result<(Vec<u8>, FileMetadata), Error> {
    let content_type = find_mime_type(path);
    let content = fs::read(path)?;
    let size = content.len();
    let metadata = FileMetadata {
        content_type: content_type,
        size,
    };
    return Ok((content, metadata));
}

struct FileMetadata {
    content_type: &'static str,
    size: usize,
}

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub http_version: String,
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

        Self {
            method: status_line[0].to_string(),
            path: status_line[1].to_string(),
            http_version: status_line[2].to_string(),
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
        Self {
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

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Settings {
    #[arg(short = 'H', long, default_value_t = String::from("0.0.0.0"))]
    host: String,

    #[arg(short = 'P', long, default_value_t = 8080)]
    port: u16,
}

fn main() {
    let settings = Settings::parse();
    let listener = TcpListener::bind(format!("{}:{}", settings.host, settings.port)).unwrap();

    println!("Listening on {}:{}", settings.host, settings.port);

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        handle_connection(&mut stream);
    }
}
