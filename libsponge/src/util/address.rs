use anyhow::{Error, Ok, Result};
use libc::{
    AF_INET, AI_ALL, AI_NUMERICHOST, AI_NUMERICSERV, addrinfo, freeaddrinfo, getaddrinfo,
    getnameinfo, in_addr, sockaddr, sockaddr_in, sockaddr_storage, socklen_t,
};

use crate::TaggedError;

use std::{mem::zeroed, ptr::null_mut};

struct GAIError(String);

impl From<String> for GAIError {
    fn from(err: String) -> Self {
        Self(err)
    }
}

impl Into<String> for GAIError {
    fn into(self) -> String {
        format!("GAIError: {}", self.0)
    }
}

pub struct RawAddr {
    pub storage: sockaddr_storage,
}

impl PartialEq for RawAddr {
    fn eq(&self, other: &Self) -> bool {
        self.storage.ss_family == other.storage.ss_family
    }
}

impl Eq for RawAddr {}

impl Default for RawAddr {
    fn default() -> Self {
        Self {
            storage: unsafe { zeroed() },
        }
    }
}

impl From<sockaddr_storage> for RawAddr {
    fn from(storage: sockaddr_storage) -> Self {
        Self { storage }
    }
}

impl RawAddr {
    pub fn as_ptr(&self) -> *const sockaddr {
        &self.storage as *const _ as *const sockaddr
    }

    pub fn as_mut_ptr(&mut self) -> *mut sockaddr {
        &mut self.storage as *mut _ as *mut sockaddr
    }
}

#[derive(PartialEq, Eq, Default)]
pub struct Address {
    pub size: socklen_t,
    addr: RawAddr,
}

impl Address {
    fn try_from_node(node: &str, service: &str, hints: &addrinfo) -> Result<Self> {
        let mut resolved_address = null_mut();
        let gai_ret = unsafe {
            getaddrinfo(
                node as *const str as *const i8,
                service as *const str as *const i8,
                hints,
                &mut resolved_address,
            )
        };
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

        struct AddrInfoGuard(*mut addrinfo);
        impl Drop for AddrInfoGuard {
            fn drop(&mut self) {
                unsafe { freeaddrinfo(self.0) };
            }
        }
        let _guard = AddrInfoGuard(resolved_address);

        unsafe {
            let addr = (*resolved_address).ai_addr as *mut sockaddr_storage;
            let size = (*resolved_address).ai_addrlen;
            Ok(Self {
                size,
                addr: RawAddr { storage: *addr },
            })
        }
    }
}

#[inline]
fn make_hints(ai_flags: i32, ai_family: i32) -> addrinfo {
    unsafe {
        let mut hints: addrinfo = zeroed();
        hints.ai_flags = ai_flags;
        hints.ai_family = ai_family;
        hints
    }
}

impl Address {
    pub fn try_from_hostname(hostname: &str, service: &str) -> Result<Self> {
        let hints = make_hints(AI_ALL, AF_INET);
        Self::try_from_node(hostname, service, &hints)
    }

    pub fn try_from_string<'a>(ip: &'a str, port: impl Into<&'a str>) -> Result<Self> {
        let hints = make_hints(AI_NUMERICHOST | AI_NUMERICSERV, AF_INET);
        Self::try_from_node(ip, port.into(), &hints)
    }
}

impl TryFrom<(*const sockaddr, usize)> for Address {
    type Error = Error;

    fn try_from(addr_and_size: (*const sockaddr, usize)) -> Result<Self> {
        let (addr, size) = addr_and_size;
        if size > size_of::<sockaddr_storage>() {
            return Err(Error::new(TaggedError::unix("invalid sockaddr size")));
        }

        if addr.is_null() {
            return Err(Error::new(TaggedError::unix("null pointer")));
        }

        unsafe {
            let mut storage: sockaddr_storage = zeroed();
            std::ptr::copy_nonoverlapping(
                addr as *const u8,
                &mut storage as *mut _ as *mut u8,
                size,
            );
            Ok(Self {
                size: size as _,
                addr: storage.into(),
            })
        }
    }
}

impl Address {
    pub fn new(size: u32, addr: sockaddr_storage) -> Self {
        Self {
            size,
            addr: RawAddr { storage: addr },
        }
    }

    pub fn ip_port(&mut self) -> Result<(String, u16)> {
        const NI_MAXHOST: usize = 1025;
        const NI_MAXSERV: usize = 32;

        let mut ip_buf = vec![0u8; NI_MAXHOST];
        let mut port_buf = vec![0u8; NI_MAXSERV];

        match unsafe {
            getnameinfo(
                &self.addr.storage as *const sockaddr_storage as *const sockaddr,
                self.size,
                ip_buf.as_mut_ptr() as _,
                ip_buf.len() as _,
                port_buf.as_mut_ptr() as _,
                port_buf.len() as _,
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
        let port: u16 = String::from_utf8_lossy(&port_buf).into_owned().parse()?;

        Ok((String::from_utf8_lossy(&ip_buf).into_owned(), port))
    }

    pub fn ip(&mut self) -> Result<String> {
        let (ip, _) = self.ip_port()?;
        Ok(ip)
    }

    pub fn port(&mut self) -> Result<u16> {
        let (_, port) = self.ip_port()?;
        Ok(port)
    }

    pub fn as_ptr(&self) -> *const sockaddr {
        self as *const Self as *const sockaddr
    }
}

impl TryInto<String> for Address {
    type Error = Error;

    fn try_into(mut self) -> Result<String> {
        let (ip, port) = self.ip_port()?;
        Ok(format!("{}:{}", ip, port))
    }
}

#[derive(PartialEq, Eq)]
pub struct IPv4NUM(pub u32);

impl TryInto<IPv4NUM> for &Address {
    type Error = Error;

    fn try_into(self) -> Result<IPv4NUM> {
        if self.addr.storage.ss_family as i32 != AF_INET
            || self.size as usize != size_of::<sockaddr_in>()
        {
            Err(Error::new(TaggedError::unix("InvalidAddress")))
        } else {
            let ipv4_addr =
                unsafe { *(&self.addr.storage as *const sockaddr_storage as *const sockaddr_in) };
            Ok(IPv4NUM(u32::from_be(ipv4_addr.sin_addr.s_addr)))
        }
    }
}

impl From<IPv4NUM> for Address {
    fn from(ip_address: IPv4NUM) -> Self {
        let ipv4_addr = sockaddr_in {
            sin_family: AF_INET as _,
            sin_port: 0,
            sin_addr: in_addr {
                s_addr: u32::to_be(ip_address.0),
            },
            sin_zero: [0; 8],
        };
        Self {
            addr: unsafe {
                RawAddr {
                    storage: *(&ipv4_addr as *const sockaddr_in as *const sockaddr_storage),
                }
            },
            size: size_of::<sockaddr_in>() as _,
        }
    }
}
