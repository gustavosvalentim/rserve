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
    let response: HttpResponse;
    let mut request = match HttpRequest::parse(stream) {
        Some(request) => request,
        None => {
            println!("Error parsing request");

            response = HttpResponse::default()
                .set_content(String::from("text/plain"), String::from(""))
                .status(500, String::from("INTERNAL SERVER ERROR"));

            stream.write_all(response.to_text().as_bytes()).unwrap();
            return;
        }
    };

    if request.path.ends_with("/") {
        let index_path = basedir.join(String::from("index.html"));
        if index_path.is_file() {
            request.path = index_path.to_string_lossy().to_string();
        }
    }

    let fs_path = basedir.join(request.path.strip_prefix("/").unwrap());
    let fs_path_in_basedir = fs_path
        .canonicalize()
        .unwrap()
        .starts_with(basedir.canonicalize().unwrap());

    println!("Filesystem path: {}", fs_path.to_str().unwrap());

    if !fs_path.exists() || !fs_path_in_basedir {
        response = HttpResponse::not_found();
    } else if fs_path.is_dir() {
        match fs_path.read_dir() {
            Ok(entries) => {
                let mut output = String::new();

                for entry in entries.flatten() {
                    let filename = entry.file_name().to_string_lossy().into_owned();
                    let entry_url = if request.path.ends_with('/') {
                        format!("{}{}", request.path, filename)
                    } else {
                        format!("{}/{}", request.path, filename)
                    };
                    let html_output = format!("<li><a href=\"{}\">{}</a></li>", entry_url, filename);

                    println!("entry_url: {}; filename: {}; html_output: {}", entry_url, filename, html_output);

                    output.push_str(html_output.as_str());
                }

                response = HttpResponse::default()
                    .set_content(String::from("text/html"), FOLDER_VIEW.replace("{dir_list}", output.as_str()))
                    .ok();
            }
            Err(_) => {
                response = HttpResponse::default()
                    .set_content(String::from("text/plain"), String::from("Failed to read directory"))
                    .status(500, String::from("INTERNAL SERVER ERROR"));
            }
        }
    } else if let Ok((content, file_metadata)) = read_file(&fs_path) {
        match String::from_utf8(content.clone()) {
            Ok(text_content) => {
                response = HttpResponse::default()
                    .set_content(file_metadata.content_type.to_string(), text_content)
                    .ok();
            }
            Err(_) => {
                response = HttpResponse::default()
                    .set_content(String::from("text/plain"), format!("Binary file ({} bytes)", file_metadata.size))
                    .ok();
            }
        }
    } else {
        response = HttpResponse::default()
            .set_content(String::from("text/plain"), String::from("Failed to read file"))
            .status(500, String::from("INTERNAL SERVER ERROR"));
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
    pub status_code: u32,
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

        text.push_str(format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text).as_str());

        for (key, value) in &self.headers {
            text.push_str(format!("{}: {}\r\n", key, value).as_str());
        }

        text.push_str("\r\n");
        text.push_str(&self.content);
        text.push_str("\r\n");

        text
    }

    pub fn set_content(mut self, mime_type: String, content: String) -> Self {
        self.content = content;
        self.headers.extend(HashMap::from([
            (String::from("Content-Type"), mime_type),
            (String::from("Content-Length"), self.content.len().to_string()),
        ]));
        self
    }

    pub fn ok(mut self) -> Self {
        self.status_code = 200;
        self.status_text = String::from("OK");
        self
    }

    pub fn status(mut self, code: u32, text: String) -> Self {
        self.status_code = code;
        self.status_text = text; 
        self
    }

    pub fn not_found() -> Self {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        HttpResponse {
            status_code: 404,
            status_text: String::from("NOT FOUND"),
            content: NOT_FOUND_VIEW.to_string(),
            headers,
        }
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
