use crate::{BufferViewList, TaggedError, system_call};

use anyhow::{Error, Result};
use libc::{c_int, c_void, iovec};

use std::{
    os::fd::RawFd,
    sync::{Arc, Mutex},
    usize,
};

const BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone)]
struct FDWrapper {
    pub fd: RawFd,
    pub eof: bool,
    pub closed: bool,
    pub read_count: usize,
    pub write_count: usize,
}

impl TryFrom<RawFd> for FDWrapper {
    type Error = Error;

    fn try_from(fd: RawFd) -> Result<Self, Self::Error> {
        if fd < 0 {
            Err(Error::msg("Invalid file descriptor"))
        } else {
            Ok(FDWrapper {
                fd,
                eof: false,
                closed: false,
                read_count: 0,
                write_count: 0,
            })
        }
    }
}

impl FDWrapper {
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            system_call("close fd", || unsafe { libc::close(self.fd) })?;
            self.closed = true;
            self.eof = true;
        }
        Ok(())
    }
}

impl Drop for FDWrapper {
    fn drop(&mut self) {
        if !self.closed {
            self.close()
                .unwrap_or_else(|err| eprintln!("Error closing file descriptor: {}", err));
        }
    }
}

pub trait FileDescriptor: Send + Sync {
    fn inner(&self) -> Arc<Mutex<FDWrapper>>;

    fn close(&self) -> Result<()> {
        self.inner().lock().unwrap().close()
    }

    fn fd(&self) -> RawFd {
        self.inner().lock().unwrap().fd
    }

    fn eof(&self) -> bool {
        self.inner().lock().unwrap().eof
    }

    fn closed(&self) -> bool {
        self.inner().lock().unwrap().closed
    }

    fn read_count(&self) -> usize {
        self.inner().lock().unwrap().read_count
    }

    fn write_count(&self) -> usize {
        self.inner().lock().unwrap().write_count
    }

    #[allow(unused)]
    fn set_blocking(&mut self, blocking: bool) -> Result<()> {
        let mut fd = self.inner().lock().unwrap().fd;
        let mut flags = system_call("fcntl", || unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
        if blocking {
            flags ^= flags & libc::O_NONBLOCK;
        } else {
            flags |= libc::O_NONBLOCK;
        }
        system_call("fcntl", || unsafe { libc::fcntl(fd, flags) })?;
        Ok(())
    }

    fn read(&mut self, limit: Option<usize>) -> Result<Vec<u8>> {
        let lmt = limit.unwrap_or(usize::MAX);
        let mut ret = Vec::with_capacity(lmt);
        self.read_into_vec(&mut ret, lmt)?;
        Ok(ret)
    }

    fn read_into_vec(&mut self, buf: &mut Vec<u8>, limit: usize) -> Result<()> {
        match self.inner().lock() {
            Ok(mut fdw) => {
                let size_to_read = BUFFER_SIZE.min(limit);
                buf.resize(size_to_read, 0);

                let bytes_read = system_call("read", || unsafe {
                    libc::read(fdw.fd, buf.as_mut_ptr() as *mut c_void, size_to_read as _)
                })?;
                if limit > 0 && fdw.read_count == 0 {
                    fdw.eof = true;
                }

                if bytes_read > size_to_read as _ {
                    return Err(Error::new(TaggedError::unix(
                        "read() read more than requested",
                    )));
                }

                buf.resize(size_to_read, 0);
                fdw.read_count += 1;
                Ok(())
            }
            Err(_) => Err(Error::new(TaggedError::unix(
                "Failed to lock file descriptor",
            ))),
        }
    }

    fn write<'a>(&mut self, buf: impl Into<BufferViewList<'a>>, write_all: bool) -> Result<usize> {
        match self.inner().lock() {
            Ok(mut fdw) => {
                let mut total_written = 0;
                let mut buf: BufferViewList = buf.into();
                loop {
                    let iovecs = buf.as_iovecs();
                    let bytes_written = system_call("writev", || unsafe {
                        libc::writev(
                            fdw.fd,
                            iovecs.as_ptr() as *const iovec,
                            iovecs.len() as c_int,
                        )
                    })?;

                    if bytes_written == 0 && buf.is_empty() {
                        return Err(Error::new(TaggedError::unix(
                            "write returned 0 given non-empty input buffer",
                        )));
                    }

                    if bytes_written as usize > buf.len() {
                        return Err(Error::new(TaggedError::unix(
                            "write() wrote more than length of input buffer",
                        )));
                    }

                    fdw.write_count += 1;
                    buf.try_remove_prefix(bytes_written as _)?;
                    total_written += bytes_written as usize;
                    if !write_all || buf.is_empty() {
                        break;
                    }
                }
                Ok(total_written)
            }
            _ => Err(Error::new(TaggedError::unix(
                "failed to acquire lock on file descriptor",
            ))),
        }
    }

    fn register_read(&self) {
        self.inner().lock().unwrap().read_count += 1;
    }

    fn register_write(&self) {
        self.inner().lock().unwrap().write_count += 1;
    }
}
