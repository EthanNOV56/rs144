use libsponge::tcp_helpers::FullStackSocket;
use std::io::stdin;

fn get_url(host: &str, path: &str) -> Result<(), String> {
    let socket = FullStackSocket::new();
    let message = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    socket.write(message.as_bytes())?;
    while !socket.eof() {
        print!("{}", socket.read()?);
    }
    socket.wait_until_closed()?;
    Ok(())
}

pub fn main() -> Result<(), String> {
    let mut host = String::new();
    let mut path = String::new();
    stdin().read_line(&mut host).expect("Failed to read host");
    stdin().read_line(&mut path).expect("Failed to read path");
    get_url(&host, &path)?;
    Ok(())
}
