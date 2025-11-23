use crate::{Address, FDAdaptor, IPv4Header, IPv4NUM, InternetDatagram, TCPSegment};

pub trait ToI {}

struct TCPOverIPv4;
impl ToI for TCPOverIPv4 {}

pub type TCPOverIPv4Adapter = FDAdaptor<TCPOverIPv4>;

fn inet_ntoa(addr: u32) -> String {
    let octets = addr.to_be_bytes();
    format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}

impl TCPOverIPv4Adapter {
    pub fn unwrap_tcp_in_ip(&mut self, ip_dgram: &InternetDatagram) -> Option<TCPSegment> {
        let dgram_src = IPv4NUM(ip_dgram.header().src);
        let dgram_dst = IPv4NUM(ip_dgram.header().dst);
        let cfg_src: IPv4NUM = (&self.cfg().source).try_into().ok()?;
        let cfg_dst: IPv4NUM = (&self.cfg().destination).try_into().ok()?;
        if !self.listen() && (dgram_dst != cfg_src || dgram_src != cfg_dst) {
            return None;
        }
        if ip_dgram.header().proto != IPv4Header::PROTO_TCP {
            return None;
        }

        let mut tcp_seg = TCPSegment::default();
        tcp_seg
            .parse(
                ip_dgram.payload().try_into().ok()?,
                ip_dgram.header().pseudo_cksum(),
            )
            .ok()?;

        if IPv4NUM(tcp_seg.header().dst_port as u32) != cfg_src
            || IPv4NUM(tcp_seg.header().src_port as u32) != cfg_dst
        {
            return None;
        }

        if self.listen() {
            if tcp_seg.header().syn && !tcp_seg.header().rst {
                let cfg = self.cfg_mut();
                cfg.source = Address::try_from_string(
                    &inet_ntoa(dgram_dst.0),
                    cfg.source.ip().ok()?.as_str(),
                )
                .ok()?;
                cfg.destination = Address::try_from_string(
                    &inet_ntoa(dgram_src.0),
                    tcp_seg.header().src_port.to_string().as_str(),
                )
                .ok()?;
                self.set_listening(false);
            } else {
                return None;
            }
        }

        if tcp_seg.header().src_port != self.cfg_mut().destination.port().ok()? {
            return None;
        }

        Some(tcp_seg)
    }

    pub fn wrap_tcp_in_ip(&mut self, tcp_seg: &mut TCPSegment) -> Option<InternetDatagram> {
        let mut ip_dgram = InternetDatagram::default();

        let src_addr = &mut self.cfg_mut().source;
        tcp_seg.header_mut().src_port = src_addr.port().ok()?;
        ip_dgram.header_mut().src = TryInto::<IPv4NUM>::try_into(src_addr as &Address).ok()?.0;

        let dst_addr = &mut self.cfg_mut().destination;
        tcp_seg.header_mut().dst_port = dst_addr.port().ok()?;
        ip_dgram.header_mut().dst = TryInto::<IPv4NUM>::try_into(dst_addr as &Address).ok()?.0;

        ip_dgram.header_mut().len = (ip_dgram.header().hlen as u16) * 4
            + (tcp_seg.header().doff as u16) * 4
            + tcp_seg.payload().len() as u16;

        *ip_dgram.payload_mut() = tcp_seg.serialize(ip_dgram.header().pseudo_cksum()).ok()?;

        Some(ip_dgram)
    }
}
