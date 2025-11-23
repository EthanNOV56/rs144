use libsponge::tcp_helpers::RS144TCPSocket;

use std::io::{Read, Write};
use std::net::TcpStream;

fn get_url(host: &str, path: &str) -> std::io::Result<()> {
    println!("Connecting to {}", host);
    let mut stream = RS144TCPSocket::connect(format!("{}:8080", host))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    println!("Sending request: {}", request);

    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    println!("{}", response);
    Ok(())
}

pub fn main() -> std::io::Result<()> {
    let host = String::from("localhost");
    let path = String::from("/hello");
    get_url(&host, &path)?;
    Ok(())
}
