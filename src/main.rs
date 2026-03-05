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
                .text_content(String::from("text/plain"), String::from(""))
                .status(500, String::from("INTERNAL SERVER ERROR"));

            stream.write_all(&response.to_bytes()).unwrap();
            return;
        }
    };

    if request.path.ends_with("/") {
        let index_path = basedir.join(
            Path::new(request.path.as_str()).join(String::from("index.html"))
        );
        if index_path.is_file() {
            request.path = index_path.to_string_lossy().to_string();
        }
    }

    let fs_path = match request.path.strip_prefix("/") {
        Some(path) => basedir.join(path),
        None => {
            response = HttpResponse::default()
                .text_content(String::from("text/plain"), String::from("Malformed URL"))
                .status(400, String::from("BAD REQUEST"));

            stream.write_all(&response.to_bytes()).unwrap();
            return;
        }
    };

    if !fs_path.exists() {
        stream.write_all(&HttpResponse::not_found().to_bytes()).unwrap();
        return;
    }

    match fs_path.canonicalize() {
        Ok(path) => {
            if !path.starts_with(basedir.canonicalize().unwrap()) {
                stream.write_all(&HttpResponse::not_found().to_bytes()).unwrap();
                return;
            }
        },
        Err(_) => {
            response = HttpResponse::default()
                .text_content(String::from("text/plain"), String::from(""))
                .status(500, String::from("INTERNAL SERVER ERROR"));

            stream.write_all(&response.to_bytes()).unwrap();

            return;
        }
    }

    println!("Filesystem path: {}", fs_path.to_str().unwrap());
    
    if fs_path.is_dir() {
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
                    .text_content(String::from("text/html"), FOLDER_VIEW.replace("{dir_list}", output.as_str()))
                    .ok();
            }
            Err(_) => {
                response = HttpResponse::default()
                    .text_content(String::from("text/plain"), String::from("Failed to read directory"))
                    .status(500, String::from("INTERNAL SERVER ERROR"));
            }
        }
    } else if let Ok((content, file_metadata)) = read_file(&fs_path) {
        match String::from_utf8(content.clone()) {
            Ok(text_content) => {
                response = HttpResponse::default()
                    .text_content(file_metadata.content_type.to_string(), text_content)
                    .ok();
            }
            Err(_) => {
                response = HttpResponse::default()
                    .bytes_content(String::from("text/plain"), content)
                    .ok();
            }
        }
    } else {
        response = HttpResponse::default()
            .text_content(String::from("text/plain"), String::from("Failed to read file"))
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

    stream.write_all(&response.to_bytes()).unwrap();

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
    };
    Ok((content, metadata))
}

struct FileMetadata {
    content_type: &'static str,
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
    pub body: Vec<u8>,
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
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        write!(&mut out, "HTTP/1.1 {} {}\r\n", self.status_code, self.status_text).unwrap();

        for (key, value) in &self.headers {
            write!(&mut out, "{}: {}\r\n", key, value).unwrap();
        }

        write!(&mut out, "\r\n").unwrap();
        out.extend_from_slice(&self.body);
        out
    }

    pub fn text_content(mut self, mime_type: String, content: String) -> Self {
        self.body = content.as_bytes().to_vec();
        self.headers.extend(HashMap::from([
            (String::from("Content-Type"), mime_type),
            (String::from("Content-Length"), self.body.len().to_string()),
        ]));
        self
    }

    pub fn bytes_content(mut self, mime_type: String, content: Vec<u8>) -> Self {
        self.body = content;
        self.headers.extend(HashMap::from([
            (String::from("Content-Type"), mime_type),
            (String::from("Content-Length"), self.body.len().to_string()),
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
            body: NOT_FOUND_VIEW.as_bytes().to_vec(),
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
