use crate::FDAdapterBase;

struct TCPOverIPv4;
type TCPOverIPv4Adapter<L> = FDAdapterBase<TCPOverIPv4, L>;

impl<L> FDAdapterBase<TCPOverIPv4, L> {
    // pub fn unwrap_tcp_in_ip(&self, ip_dgram: &InternetDatagram)
    // pub fn wrap_tcp_in_ip()
}
