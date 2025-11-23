use crate::{Address, BufferViewList, FDWrapper, FileDescriptor, RawAddr, system_call};

use anyhow::{Error, Result};
use libc::{
    AF_INET, AF_UNIX, MSG_TRUNC, SHUT_RD, SHUT_RDWR, SHUT_WR, SO_DOMAIN, SO_REUSEADDR, SO_TYPE,
    SOCK_DGRAM, SOCK_STREAM, SOL_SOCKET, accept, bind, c_int, c_void, connect, getpeername,
    getsockname, getsockopt, iovec, listen, msghdr, recvfrom, sendmsg, setsockopt, shutdown,
    sockaddr, socket, socklen_t,
};
use thiserror::Error;

use std::{
    mem::{size_of, zeroed},
    os::fd::RawFd,
};

#[derive(Debug, Error, Clone)]
pub enum SocketError {
    #[error("Socket domain mismatch.")]
    SocketDomainMismatch,
    #[error("Socket type mismatch.")]
    SocketTypeMismatch,
    #[error("Unknown shutdown type.")]
    UnknownShutdownType,
    #[error("Datagram is oversized for ({0})")]
    DatagramOversized(String),
}

pub trait Socket: FileDescriptor {
    fn get_addr<F>(&self, func_name: &str, func: F) -> Result<Address>
    where
        F: FnOnce(i32, *mut sockaddr, *mut socklen_t) -> i32,
    {
        let mut addr = RawAddr::default();
        let size = std::mem::size_of::<RawAddr>();
        system_call(func_name, || func(self.fd(), addr.as_mut_ptr(), size as _))?;
        Ok(Address::new(size as _, addr.storage))
    }

    fn try_build(fd: Option<FDWrapper>, domain: i32, ty: i32) -> Result<FDWrapper> {
        match fd {
            None => {
                let fd = system_call("socket", || unsafe { socket(domain, ty, 0) })?;
                FDWrapper::try_from(fd)
            }
            Some(fd) => {
                let mut val: c_int = unsafe { zeroed() };
                let mut len: socklen_t = size_of::<i32>() as _;
                system_call("getsockopt", || unsafe {
                    getsockopt(
                        fd.fd,
                        SOL_SOCKET,
                        SO_DOMAIN,
                        &mut val as *mut c_int as *mut c_void,
                        &mut len as *mut socklen_t,
                    )
                })?;
                if len as usize != size_of::<c_int>() || val as i32 != domain {
                    return Err(Error::new(SocketError::SocketDomainMismatch));
                }

                len = size_of::<c_int>() as _;
                system_call("getsockopt", || unsafe {
                    getsockopt(
                        fd.fd,
                        SOL_SOCKET,
                        SO_TYPE,
                        &mut val as *mut c_int as *mut c_void,
                        &mut len as *mut socklen_t,
                    )
                })?;
                if len as usize != size_of::<c_int>() || val as i32 != ty {
                    return Err(Error::new(SocketError::SocketTypeMismatch));
                }

                FDWrapper::try_from(fd.fd)
            }
        }
    }

    fn set_opt<T>(&mut self, level: i32, opt: i32, opt_val: T) -> Result<()> {
        system_call("setsockopt", || unsafe {
            setsockopt(
                self.fd() as _,
                level,
                opt,
                &opt_val as *const T as *const c_void,
                size_of::<T>() as socklen_t,
            )
        })?;
        Ok(())
    }

    #[inline]
    fn connect(&mut self, addr: &Address) -> Result<()> {
        system_call("connect", || unsafe {
            connect(self.fd() as _, addr.as_ptr(), addr.size)
        })?;
        Ok(())
    }

    #[inline]
    fn bind(&mut self, addr: &Address) -> Result<()> {
        system_call("bind", || unsafe {
            bind(self.fd() as _, addr.as_ptr(), addr.size)
        })?;
        Ok(())
    }

    #[inline]
    fn shutdown(&mut self, how: i32) -> Result<()> {
        system_call("shutdown", || unsafe { shutdown(self.fd() as _, how) })?;
        match how {
            SHUT_RD => self.register_read(),
            SHUT_WR => self.register_write(),
            SHUT_RDWR => {
                self.register_read();
                self.register_write();
            }
            _ => return Err(Error::from(SocketError::UnknownShutdownType)),
        }
        Ok(())
    }

    #[inline]
    fn local_addr(&self) -> Result<Address> {
        self.get_addr("getsockname", |i, j, k| unsafe { getsockname(i, j, k) })
    }

    #[inline]
    fn peer_addr(&self) -> Result<Address> {
        self.get_addr("getpeername", |i, j, k| unsafe { getpeername(i, j, k) })
    }

    #[inline]
    fn set_reuseaddr(&mut self) -> Result<()> {
        self.set_opt(SOL_SOCKET, SO_REUSEADDR, true as i32)
    }
}

pub trait TCPSocket: Socket {
    fn try_from_fd(fd: Option<FDWrapper>) -> Result<FDWrapper> {
        Self::try_build(fd, AF_INET, SOCK_STREAM)
    }

    fn try_default() -> Result<FDWrapper> {
        Self::try_from_fd(None)
    }

    fn listen(&self, backlog: Option<i32>) -> Result<()> {
        system_call("listen", || unsafe {
            listen(self.fd(), backlog.unwrap_or(16))
        })?;
        Ok(())
    }

    fn accept(&mut self) -> Result<FDWrapper> {
        self.register_read();
        let raw = system_call("accept", || unsafe {
            accept(self.fd(), std::ptr::null_mut(), std::ptr::null_mut())
        })?;

        Self::try_from_fd(FDWrapper::try_from(raw).ok())
    }
}

#[derive(Default)]
pub struct RcvdDatagram {
    src_addr: Address,
    pub payload: Vec<u8>,
}

fn send_helper(
    fd: RawFd,
    des_addr: *mut sockaddr,
    des_addr_len: socklen_t,
    payload: &BufferViewList,
) -> Result<()> {
    let mut iovecs = payload.as_iovecs();
    let mut message = msghdr {
        msg_name: des_addr as *mut _,
        msg_namelen: des_addr_len,
        msg_iov: iovecs.as_mut_ptr() as *mut iovec,
        msg_iovlen: iovecs.len(),
        msg_control: std::ptr::null_mut(),
        msg_controllen: 0,
        msg_flags: 0,
    };
    let byte_sent = system_call("sendmsg", || unsafe {
        sendmsg(fd, &mut message as *mut _, 0)
    })?;
    if byte_sent as usize != payload.len() {
        Err(Error::from(SocketError::DatagramOversized(String::from(
            "sendmsg()",
        ))))
    } else {
        Ok(())
    }
}

pub trait UDPSocket: Socket {
    fn try_from_fd(fd: Option<FDWrapper>) -> Result<FDWrapper> {
        Self::try_build(fd, AF_INET, SOCK_DGRAM)
    }

    fn try_default() -> Result<FDWrapper> {
        Self::try_build(None, AF_INET, SOCK_DGRAM)
    }

    fn recv(&mut self, mtu: Option<usize>) -> Result<RcvdDatagram> {
        let mut dg = RcvdDatagram::default();
        self.recv_into_datagram(&mut dg, mtu)?;
        Ok(dg)
    }

    fn recv_into_datagram(
        &mut self,
        datagram: &mut RcvdDatagram,
        mtu: Option<usize>,
    ) -> Result<()> {
        let mtu = mtu.unwrap_or(65536);
        let mut addr = RawAddr::default();
        datagram.payload.resize(mtu, 0);
        let fromlen = size_of::<Address>();
        let recv_len = system_call("recvfrom", || unsafe {
            recvfrom(
                self.fd(),
                datagram.payload.as_mut_ptr() as _,
                mtu as _,
                MSG_TRUNC,
                addr.as_mut_ptr(),
                fromlen as _,
            )
        })?;

        if recv_len as usize > mtu {
            return Err(Error::from(SocketError::DatagramOversized(String::from(
                "recvfrom",
            ))));
        }

        self.register_read();
        datagram.src_addr = Address::new(fromlen as _, addr.storage);
        datagram.payload.resize(recv_len as _, 0);
        Ok(())
    }

    fn send_to(&mut self, des: &mut Address, payload: &BufferViewList) -> Result<()> {
        send_helper(self.fd(), des as *mut Address as *mut _, des.size, payload)?;
        self.register_write();
        Ok(())
    }

    fn send(&mut self, payload: &BufferViewList) -> Result<()> {
        send_helper(self.fd(), 0 as *mut Address as *mut _, 0, payload)?;
        self.register_write();
        Ok(())
    }
}

pub trait LSSocket: Socket {
    fn try_from_fd(fd: Option<FDWrapper>) -> Result<FDWrapper> {
        Self::try_build(fd, AF_UNIX, SOCK_STREAM)
    }
}
