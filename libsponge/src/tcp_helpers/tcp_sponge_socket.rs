use std::{sync::atomic::AtomicBool, thread};

use crate::{
    EventLoop, LSSocket, TCPConnection, TCPOverIPv4OverEthernetAdapter, TCPOverIPv4OverTunFdAdapter,
};

pub struct TCPSpongeSocket<A> {
    thread_data: LSSocket<A>,
    dgram_adapter: A,
    tcp_connection: Option<TCPConnection>,
    event_loop: EventLoop,
    // tcp_thread: thread,
    abort: AtomicBool,
    inbound_shutdown: bool,
    outbound_shutdown: bool,
    fully_acked: bool,
}

pub type TCPOverIPv4SpongeSocket = TCPSpongeSocket<TCPOverIPv4OverTunFdAdapter>;

pub type RS144TCPSocket = TCPOverIPv4SpongeSocket;

pub type TCPOverIPv4OverEthernetSpongeSocket = TCPSpongeSocket<TCPOverIPv4OverEthernetAdapter>;

#[derive(Default)]
pub struct FullStackSocket {
    eof_flag: bool,
}

impl FullStackSocket {
    pub fn new() -> Self {
        FullStackSocket {
            ..Default::default()
        }
    }

    pub fn connect(&self, addr: &str) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn read(&self) -> Result<u8, String> {
        // Implementation goes here
        Ok(0)
    }

    pub fn wait_until_closed(&self) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn eof(&self) -> bool {
        self.eof_flag
    }
}
