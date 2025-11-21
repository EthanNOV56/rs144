use crate::{Address, BufferViewList, FileDescriptor, NakedFileDescriptor, RawAddr, system_call};

use anyhow::Result;
use libc::{
    AF_INET, AF_UNIX, MSG_TRUNC, SHUT_RD, SHUT_RDWR, SHUT_WR, SO_DOMAIN, SO_REUSEADDR, SO_TYPE,
    SOCK_DGRAM, SOCK_STREAM, SOL_SOCKET, accept, bind, c_int, c_void, connect, getpeername,
    getsockname, getsockopt, iovec, listen, msghdr, recvfrom, sendmsg, setsockopt, shutdown,
    sockaddr, socket, socklen_t,
};

use std::{mem::size_of, os::fd::RawFd};

struct SocketT;
pub type Socket<A, P> = FileDescriptor<A, SocketT, P>;

impl<A, P> Socket<A, P> {
    fn get_addr<F>(&self, func_name: &str, func: F) -> Result<Address>
    where
        F: FnOnce(i32, *mut sockaddr, *mut socklen_t) -> i32,
    {
        let mut addr = RawAddr::default();
        let size = std::mem::size_of::<RawAddr>();
        system_call(func_name, || func(self.fd(), addr.as_mut_ptr(), size as _));
        Ok(Address::new(size as _, addr.storage))
    }

    fn try_build(fd: Option<NakedFileDescriptor>, domain: i32, ty: i32) -> Result<Socket<A, P>> {
        match fd {
            None => {
                let fd = system_call("socket", || unsafe { socket(domain, ty, 0) })?;
                Ok(Socket::from(fd as RawFd))
            }
            Some(fd) => {
                let mut val: c_void;
                let mut len: socklen_t = size_of::<i32>() as _;
                system_call("getsockopt", || unsafe {
                    getsockopt(
                        fd.fd() as _,
                        SOL_SOCKET,
                        SO_DOMAIN,
                        &mut val as *mut c_void,
                        &mut len as *mut socklen_t,
                    )
                })?;
                if len != sizeof::<c_int>() || val as i32 != domain {
                    return Err(Error::new(ErrorKind::Other, "socket domain mismatch"));
                }

                len = sizeof::<c_int>() as _;
                system_call("getsockopt", || unsafe {
                    getsockopt(
                        fd.fd() as _,
                        SOL_SOCKET,
                        SO_TYPE,
                        &mut val as *mut c_void,
                        &mut len as *mut socklen_t,
                    )
                })?;
                if len != sizeof::<c_int>() || val as i32 != ty {
                    return Err(Error::new(ErrorKind::Other, "socket type mismatch"));
                }

                Ok(Socket::from(fd as _))
            }
        }
    }

    fn set_opt<T>(&mut self, level: i32, opt: i32, opt_val: T) -> Result<()> {
        system_call("setsockopt", || unsafe {
            setsockopt(
                self.fd.fd() as _,
                level,
                opt,
                &opt_val as *const T as *const c_void,
                size_of::<T>() as socklen_t,
            )
        })?;
        Ok(())
    }

    #[inline]
    pub fn connect(&mut self, address: &Address) -> Result<()> {
        system_call("connect", || unsafe {
            connect(
                self.fd.fd() as _,
                address.as_ptr(),
                address.len() as socklen_t,
            )
        })
    }

    #[inline]
    pub fn bind(&mut self, addr: &Address) -> Result<()> {
        system_call("bind", || unsafe {
            bind(self.fd.fd() as _, addr.as_ptr(), addr.len() as socklen_t)
        })?;
        Ok(())
    }

    #[inline]
    pub fn shutdown(&mut self, how: i32) -> Result<()> {
        system_call("shutdown", || unsafe { shutdown(self.fd.fd() as _, how) })?;
        match how {
            SHUT_RD => self.register_read(),
            SHUT_WR => self.register_write(),
            SHUT_RDWR => self.register_read().and_then(|_| self.register_write()),
        }
        Ok(())
    }

    #[inline]
    pub fn local_addr(&self) -> Result<Address> {
        self.get_addr("getsockname", |i, j, k| unsafe { getsockname(i, j, k) })
    }

    #[inline]
    pub fn peer_addr(&self) -> Result<Address> {
        self.get_addr("getpeername", |i, j, k| unsafe { getpeername(i, j, k) })
    }

    #[inline]
    pub fn set_reuseaddr(&mut self) -> Result<()> {
        self.set_opt(SOL_SOCKET, SO_REUSEADDR, true as i32)
    }
}

struct TCP;
pub type TCPSocket<A> = Socket<A, TCP>;
struct UDP;
pub type UDPSocket<A> = Socket<A, UDP>;
struct LS;
pub type LSSocket<A> = Socket<A, LS>;

impl<A> TCPSocket<A> {
    fn try_from_fd(fd: NakedFileDescriptor) -> Result<Self> {
        Self::try_build(Some(fd), AF_INET, SOCK_STREAM)
    }

    pub fn try_default() -> Result<Self> {
        Self::try_build(None, AF_INET, SOCK_STREAM)
    }

    pub fn listen(&self, backlog: Option<i32>) -> Result<()> {
        system_call("listen", || unsafe {
            listen(self.fd(), backlog.unwrap_or(16))
        })
    }

    pub fn accept(&mut self) -> Result<TCPSocket<A>> {
        self.register_read();
        let raw = system_call("accept", || unsafe {
            accept(self.fd(), std::ptr::null_mut(), std::ptr::null_mut())
        })?;

        Self::try_from_fd(NakedFileDescriptor::from(raw))
    }
}

#[derive(Default)]
struct RcvdDatagram {
    src_addr: Address,
    pub payload: Vec<u8>,
}

fn send_helper(
    fd: RawFd,
    des_addr: &sockaddr,
    des_addr_len: socklen_t,
    payload: &BufferViewList,
) -> Result<()> {
    let iovecs = payload.as_iovecs();
    let mut message = msghdr {
        msg_name: des_addr as *const _ as *mut _,
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
    if byte_sent != payload.len() {
        Err(Error::new(
            ErrorKind::Other,
            "datagram payload too big for sendmsg()",
        ))
    } else {
        Ok(())
    }
}

impl<A> UDPSocket<A> {
    fn try_from_fd(fd: NakedFileDescriptor) -> Result<Self> {
        Self::try_build(Some(fd), AF_INET, SOCK_DGRAM)
    }

    pub fn try_default() -> Result<Self> {
        Self::try_build(None, AF_INET, SOCK_DGRAM)
    }

    pub fn recv(&self, mtu: Option<usize>) -> Result<RcvdDatagram> {
        let mut dg = RcvdDatagram::default();
        self.recv_into_datagram(&mut dg, mtu)?
    }

    pub fn recv_into_datagram(
        &self,
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

        if recv_len as _ > mtu {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "recvfrom (oversized datagram)",
            ));
        }

        self.register_read();
        datagram.src_addr = Address::new(fromlen, addr);
        datagram.payload.resize(recv_len, 0);
        Ok(())
    }

    pub fn send_to(&self, des: &Address, payload: &BufferViewList) -> Result<()> {
        send_helper(
            self.fd(),
            des as *const Address as *const c_void,
            des.addr_len(),
            payload,
        )?;
        self.register_write();
        Ok(())
    }

    pub fn send(&self, payload: &BufferViewList) -> Result<()> {
        send_helper(self.fd(), 0, 0, payload)?;
        self.register_write();
        Ok(())
    }
}

impl<A> LSSocket<A> {
    fn try_from_fd(fd: NakedFileDescriptor) -> Result<Self> {
        Self::try_build(Some(fd), AF_UNIX, SOCK_STREAM)
    }
}
