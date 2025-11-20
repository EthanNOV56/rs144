use anyhow::{Error, Ok, Result};
use itertools::structs;
use libc::{
    AF_INET, AI_ALL, AI_NUMERICHOST, AI_NUMERICSERV, addrinfo, freeaddrinfo, getaddrinfo,
    getnameinfo, sockaddr, sockaddr_in, sockaddr_storage, socklen_t,
};

use crate::TaggedError;

struct GAIError(String);

impl From<String> for GAIError {
    fn from(err: String) -> Self {
        Self(err)
    }
}

#[derive(PartialEq, Eq)]
pub struct RawAddr {
    pub storage: sockaddr_storage,
}

impl From<sockaddr_storage> for RawAddr {
    fn from(storage: sockaddr_storage) -> Self {
        Self { storage }
    }
}

impl RawAddr {
    pub fn new() -> Self {
        unsafe {
            Self {
                storage: std::mem::zeroed(),
            }
        }
    }

    pub fn as_ptr(&self) -> *const sockaddr {
        &self.storage as *const _ as *const sockaddr
    }

    pub fn as_mut_ptr(&mut self) -> *mut sockaddr {
        &mut self.storage as *mut _ as *mut sockaddr
    }
}

#[derive(PartialEq, Eq)]
pub struct Address {
    size: socklen_t,
    addr: RawAddr,
    ip: Option<String>,
    port: Option<u16>,
}

impl Address {
    fn try_from_node(node: &str, service: &str, hints: &addrinfo) -> Result<Self> {
        let mut resolved_address = ptr::null_mut();
        let gai_ret = unsafe { getaddrinfo(node, service, hints, &mut resolved_address) };
        if gai_ret != 0 {
            return Err(Error::new(TaggedError::unix(GAIError::from(
                "getaddrinfo".to_string(),
            ))));
        }
        if resolved_address.is_null() {
            return Err(Error::new(TaggedError::unix(GAIError::from(
                "No address found".to_string(),
            ))));
        }
        let addrinfo_dropper = |x| unsafe { freeaddrinfo(x) };
        let wrapped_address = Box::new(addrinfo_dropper(resolved_address));
        let addr = RawAddr::new();
        let size = wrapped_address.ai_addrlen;
        Ok(Self {
            size,
            addr,
            ip: None,
            port: None,
        })
    }
}

#[inline]
fn make_hints(ai_flags: i32, ai_family: i32) -> addrinfo {
    let mut hints = mem::zeroed();
    hints.ai_flags = ai_flags;
    hints.ai_family = ai_family;
    hints
}

impl Address {
    pub fn try_from_hostname(hostname: &str, service: &str) -> Result<Self> {
        let hints = make_hints(AI_ALL, AF_INET);
        Self::try_from_node(hostname, service, &hints)
    }

    pub fn try_from_string(ip: &str, port: impl Into<&str>) -> Result<Self> {
        let hints = make_hints(AI_NUMERICHOST | AI_NUMERICSERV, AF_INET);
        Self::try_from_node(ip, port.into(), &hints)
    }
}

impl TryFrom<*const sockaddr> for Address {
    type Error = Error;

    fn try_from(addr: *const sockaddr, size: usize) -> Result<Self> {
        if size > mem::size_of::<sockaddr_storage>() {
            return Err(Error::new("invalid sockaddr size"));
        }

        if addr.is_null() {
            return Err(Error::new("null pointer"));
        }

        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            std::ptr::copy_nonoverlapping(
                addr as *const u8,
                &mut storage as *mut _ as *mut u8,
                size,
            );
        }
        Ok(Self {
            size,
            addr: storage.into(),
            ip: None,
            port: None,
        })
    }
}

impl Address {
    pub fn ip_port(&mut self) -> Result<(String, u16)> {
        const NI_MAXHOST: usize = 1025;
        const NI_MAXSERV: usize = 32;

        let mut ip_buf = vec![0u8; NI_MAXHOST];
        let mut port_buf = vec![0u8; NI_MAXSERV];

        match unsafe {
            getnameinfo(
                self.into(),
                socklen_t,
                ip_buf.as_mut_ptr(),
                ip_buf.len(),
                port_buf.as_mut_ptr(),
                port_buf.len(),
                AI_NUMERICHOST | AI_NUMERICSERV,
            )
        } {
            0 => {}
            _ => {
                return Err(Error::new(TaggedError::unix(GAIError::from(
                    "getnameinfo".to_string(),
                ))));
            }
        }

        Ok((
            String::from_utf8_lossy(&ip_buf).into_owned(),
            u16::from_str(&String::from_utf8_lossy(&port_buf).into_owned()).unwrap(),
        ))
    }

    pub fn ip(&self) -> Result<String> {
        let (ip, _) = self.ip_port()?;
        Ok(ip)
    }

    pub fn port(&self) -> Result<u16> {
        let (_, port) = self.ip_port()?;
        Ok(port)
    }
}

impl TryInto<String> for Address {
    type Error = Error;

    fn try_into(mut self) -> Result<String> {
        let (ip, port) = self.ip_port()?;
        Ok(format!("{}:{}", ip, port))
    }
}

pub struct IPv4NUM(u32);

impl TryInto<IPv4NUM> for Address {
    type Error = Error;

    fn try_into(mut self) -> Result<IPv4NUM> {
        if self.addr.storage.ss_family != AF_INET || self.size != mem::size_of::<sockaddr_in>() {
            Err(Error::InvalidAddress)
        } else {
            let ipv4_addr = unsafe { *(self.addr.storage.as_ptr() as *const sockaddr_in) };
            Ok(IPv4NUM(u32::from_be(ipv4_addr.sin_addr.s_addr)))
        }
    }
}

impl From<IPv4NUM> for Address {
    fn from(ip_address: IPv4NUM) -> Self {
        let mut ipv4_addr = sockaddr_in {
            sin_family: AF_INET,
            sin_port: 0,
            sin_addr: in_addr {
                s_addr: u32::to_be(ip_address.0),
            },
            sin_zero: [0; 8],
        };
        Self {
            addr: ipv4_addr,
            size: mem::size_of::<sockaddr_in>(),
            ip: Some(ipv4_addr.sin_addr.s_addr),
            port: None,
        }
    }
}
