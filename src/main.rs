use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

const NOT_FOUND_VIEW: &str = include_str!("pages/404.html");
const FOLDER_VIEW: &str = include_str!("pages/folder_view.html");

pub fn handle_connection(mut stream: &TcpStream) {
    let cwdir = std::env::current_dir().unwrap();
    let basedir = Path::new(cwdir.to_str().unwrap());
    let mut response = HttpResponse::default();
    let mut request = match HttpRequest::parse(stream) {
        Some(request) => request,
        None => {
            println!("Error parsing request");

            response.status_code = 500;
            response.status_text = String::from("INTERNAL SERVER ERROR");
            stream.write_all(response.to_text().as_bytes()).unwrap();
            return;
        }
    };

    if request.path == "/" {
        let index_path = basedir.join(String::from("index.html"));
        if index_path.is_file() {
            request.path = index_path.to_string_lossy().to_string();
        }
    }

    let fs_path = basedir.join(request.path.strip_prefix("/").unwrap());

    println!("Filesystem path: {}", fs_path.to_str().unwrap());

    if !fs_path.exists() {
        response.content = NOT_FOUND_VIEW.to_string();
        response.status_code = 404;
        response.status_text = String::from("NOT FOUND");
    }

    if fs_path.is_dir() {
        let mut output = String::new();

        for entry in fs_path.read_dir().unwrap().flatten() {
            let filename = entry.file_name().to_string_lossy().into_owned();
            let parts = [request.path.as_str(), filename.as_str()];
            let entry_url = parts.join("/");
            let html_output = format!("<li><a href=\"{}\">{}</a></li>", entry_url, filename);

            println!("entry_url: {}; filename: {}; html_output: {}", entry_url, filename, html_output);

            output.push_str(html_output.as_str());
        }

        response.status_code = 200;
        response.status_text = String::from("OK");
        response.content = FOLDER_VIEW.replace("{dir_list}", output.as_str());
        response.headers.insert(
            String::from("Content-Length"),
            response.content.len().to_string(),
        );
        response
            .headers
            .insert(String::from("Content-Type"), String::from("text/html"));
    } else if let Ok((content, file_metadata)) = read_file(&fs_path) {
        response.status_code = 200;
        response.status_text = String::from("OK");
        response.content = String::from_utf8(content).unwrap();
        response.headers.insert(
            String::from("Content-Length"),
            file_metadata.size.to_string(),
        );
        response.headers.insert(
            String::from("Content-Type"),
            file_metadata.content_type.to_string(),
        );
    } else {
        response.status_code = 500;
        response.status_text = String::from("INTERNAL SERVER ERROR");
    }

    println!(
        "Request {} {} {} - {} {}",
        request.method,
        request.path,
        request.http_version,
        response.status_code,
        response.status_text
    );

    stream.write_all(response.to_text().as_bytes()).unwrap();

    println!(
        "Response {} {} {}",
        request.http_version, response.status_code, response.status_text
    );
}

fn find_mime_type(path: &Path) -> &'static str {
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
            _ => "text/plain",
        }
    }

    mime_type
}

fn read_file(path: &Path) -> Result<(Vec<u8>, FileMetadata), std::io::Error> {
    let content = fs::read(path)?;
    let metadata = FileMetadata {
        content_type: find_mime_type(path),
        size: content.len(),
    };
    Ok((content, metadata))
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
    pub fn parse(stream: &TcpStream) -> Option<Self> {
        let buf_reader = BufReader::new(stream);
        let request_lines: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        if request_lines.is_empty() {
            return None;
        }

        let status_line: Vec<_> = request_lines[0].split(' ').collect();

        Some(Self {
            method: status_line[0].to_string(),
            path: status_line[1].to_string(),
            http_version: status_line[2].to_string(),
        })
    }
}

#[derive(Debug, Default)]
pub struct HttpResponse {
    pub status_code: i32,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub content: String,
}

impl HttpResponse {
    /**
     * Returns the response in text format.
     *
     * Examples:
     *
     * ```
     * let response = HttpResponse::new();
     * response.status_code = 400;
     * response.status_text = "BAD REQUEST";
     *
     * let response_parts = response.to_text().split("\r\n").collect()[0];
     * let status_line = response_parts.split(' ').collect();
     *
     * assert_eq!("400", status_line[1]);
     * ```
     */
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

    println!("Listening on http://{}:{}", settings.host, settings.port);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(&stream);
    }
}
