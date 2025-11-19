use std::{
    ffi::c_void,
    os::unix::io::RawFd,
    sync::{Arc, Mutex},
    usize,
};

const BUFFER_SIZE: usize = 1024 * 1024;

use anyhow::Result;

use crate::{BufferViewList, TaggedError, system_call};

#[derive(Clone)]
struct FDWrapper {
    pub fd: RawFd,
    pub eof: bool,
    pub closed: bool,
    pub read_count: usize,
    pub write_count: usize,
}

impl FDWrapper {
    pub fn new(fd: RawFd) -> Self {
        if fd < 0 {
            panic!("Invalid file descriptor");
        }
        FDWrapper {
            fd,
            eof: false,
            closed: false,
            read_count: 0,
            write_count: 0,
        }
    }

    pub fn close(&mut self) -> Result<(), TaggedError> {
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

#[derive(Clone)]
pub struct FileDescriptor {
    internal_fd: Arc<Mutex<FDWrapper>>,
}

impl FileDescriptor {
    fn register_read(&mut self) {
        self.internal_fd.lock().unwrap().read_count += 1;
    }

    fn register_write(&mut self) {
        self.internal_fd.lock().unwrap().write_count += 1;
    }
}

impl FileDescriptor {
    pub fn new(fd: RawFd) -> Self {
        Self {
            internal_fd: Arc::new(Mutex::new(FDWrapper::new(fd))),
        }
    }

    pub fn close(&mut self) {
        self.internal_fd.lock().unwrap().close();
    }

    pub fn fd(&self) -> i32 {
        self.internal_fd.lock().unwrap().fd
    }

    pub fn eof(&self) -> bool {
        self.internal_fd.lock().unwrap().eof
    }

    pub fn closed(&self) -> bool {
        self.internal_fd.lock().unwrap().closed
    }

    pub fn read_count(&self) -> usize {
        self.internal_fd.lock().unwrap().read_count
    }

    pub fn write_count(&self) -> usize {
        self.internal_fd.lock().unwrap().write_count
    }

    pub fn set_blocking(&mut self, blocking: bool) -> Result<(), TaggedError> {
        let mut fd = self.internal_fd.lock().unwrap().fd;
        let flags = system_call("fcntl", || unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
        if blocking {
            flags ^= (flags & libc::O_NONBLOCK);
        } else {
            flags |= libc::O_NONBLOCK;
        }
        system_call("fcntl", || unsafe { libc::fcntl(fd, flags) })?;
        Ok(())
    }
}

impl FileDescriptor {
    pub fn read(&mut self, limit: Option<usize>) -> Result<Vec<u8>, TaggedError> {
        let lmt = limit.unwrap_or(usize::MAX);
        let mut ret = Vec::with_capacity(lmt);
        self.read_into_vec(&mut ret, lmt)?;
        Ok(ret)
    }

    pub fn read_into_vec(&mut self, buf: &mut Vec<u8>, limit: usize) -> Result<(), TaggedError> {
        let mut fdw = self.internal_fd.lock().unwrap();
        let size_to_read = BUFFER_SIZE.min(limit);
        buf.resize(size_to_read, 0);

        let bytes_read = system_call("read", || unsafe {
            libc::read(fdw.fd, buf.as_mut_ptr() as *mut c_void, size_to_read as _)
        })?;
        if limit > 0 && fdw.read_count == 0 {
            fdw.eof = true;
        }

        if fdw.read_count > size_to_read {
            return Err(TaggedError::new(
                "read() read more than requested",
                std::io::Error::last_os_error(),
            ));
        }

        buf.resize(size_to_read, 0);
        self.register_read();
        Ok(())
    }
}

impl FileDescriptor {
    // //! Write a string, possibly blocking until all is written
    // size_t write(const char *str, const bool write_all = true) { return write(BufferViewList(str), write_all); }

    // //! Write a string, possibly blocking until all is written
    // size_t write(const std::string &str, const bool write_all = true) { return write(BufferViewList(str), write_all); }

    // //! Write a buffer (or list of buffers), possibly blocking until all is written
    // size_t write(BufferViewList buffer, const bool write_all = true);
    pub fn write<'a>(
        &mut self,
        buf: impl Into<BufferViewList<'a>>,
        write_all: bool,
    ) -> Result<usize, TaggedError> {
        let mut fdw = self.internal_fd.lock().unwrap();
        let mut total_written = 0;
        let mut buf: BufferViewList = buf.into();
        loop {
            let iovecs = buf.as_iovecs();
            let bytes_written = system_call("writev", || unsafe {
                libc::writev(fdw, iovecs.as_ptr(), iovecs.len() as libc::c_int)
            })?;
            if bytes_written == 0 && buf.len() != 0 {
                return Err(TaggedError::new(
                    "write returned 0 given non-empty input buffer",
                    std::io::Error::last_os_error(),
                ));
            }

            if bytes_written as usize > buf.len() {
                return Err(TaggedError::new(
                    "write() wrote more than length of input buffer",
                    std::io::Error::last_os_error(),
                ));
            }

            self.register_write();
            buf.try_remove_prefix(bytes_written as _)?;
            if !write_all || buf.len() == 0 {
                break;
            }
        }
        Ok(total_written)
    }
}
