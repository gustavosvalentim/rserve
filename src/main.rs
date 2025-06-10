mod http;

use std::net::{TcpListener};

struct Settings {
    host: String,
    port: u16,
}

impl Settings {
    fn new() -> Self {
        Self{
            host: String::from("0.0.0.0"),
            port: 8080,
        }
    }
}

fn main() {
    let settings = Settings::new();
    let listener = TcpListener::bind(format!("{}:{}", settings.host, settings.port)).unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        
        http::handle_http_request(&mut stream);
    }
}