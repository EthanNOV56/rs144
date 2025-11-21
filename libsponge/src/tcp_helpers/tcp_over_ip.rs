use crate::{FDAdaptor, InternetDatagram, TCPSegment};

pub trait ToI {}

struct TCPOverIPv4;
impl ToI for TCPOverIPv4 {}

pub type TCPOverIPv4Adapter = FDAdaptor<TCPOverIPv4>;

impl TCPOverIPv4Adapter {
    pub fn unwrap_tcp_in_ip(&self, ip_dgram: &InternetDatagram) -> Option<TCPSegment> {}
    pub fn wrap_tcp_in_ip(&self, tcp_seg: &TCPSegment) -> InternetDatagram {}
}
