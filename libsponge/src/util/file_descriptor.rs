use std::{
    io::{Error, Read, Write},
    os::unix::io::RawFd,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct FDWrapper {
    pub fd: RawFd,
    pub eof: bool,
    pub closed: bool,
    pub read_count: u32,
    pub write_count: u32,
}

impl FDWrapper {
    pub fn new(fd: RawFd) -> Self {
        if fd < 0 {
            panic!("Invalid file descriptor");
        }
        FDWrapper {
            fd,
            ..Default::default()
        }
    }

    pub fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.eof = true;
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

impl Drop for FDWrapper {
    fn drop(&mut self) {
        if !self.closed {
            self.close();
        }
    }
}

#[derive(Debug, Default, Clone)]
struct FileDescriptor {
    internal_fd: Arc<Mutex<FDWrapper>>,
}

impl FileDescriptor {
    pub fn new(fd: RawFd) -> Self {
        FileDescriptor {
            internal_fd: Arc::new(Mutex::new(FDWrapper::new(fd))),
            ..Default::default()
        }
    }

    pub fn close(&mut self) {
        self.internal_fd.lock().unwrap().close();
    }

    fn register_read(&mut self) {
        self.internal_fd.lock().unwrap().read_count += 1;
    }

    fn register_write(&mut self) {
        self.internal_fd.lock().unwrap().write_count += 1;
    }

    pub fn fd_num(&self) -> i32 {
        self.internal_fd.lock().unwrap().fd
    }

    pub fn eof(&self) -> bool {
        self.internal_fd.lock().unwrap().eof
    }

    pub fn closed(&self) -> bool {
        self.internal_fd.lock().unwrap().closed
    }

    pub fn read_count(&self) -> u32 {
        self.internal_fd.lock().unwrap().read_count
    }

    pub fn write_count(&self) -> u32 {
        self.internal_fd.lock().unwrap().write_count
    }

    pub fn set_blocking(&mut self, blocking: bool) -> std::io::Result<()> {
        let guard = self.internal_fd.lock().unwrap();
        let mut flags = unsafe { libc::fcntl(guard.fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(Error::last_os_error());
        }
        if blocking {
            flags &= !libc::O_NONBLOCK;
        } else {
            flags |= libc::O_NONBLOCK;
        }
        let ret = unsafe { libc::fcntl(guard.fd, libc::F_SETFL, flags) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

// //! Read up to `limit` bytes
// std::string read(const size_t limit = std::numeric_limits<size_t>::max());

// //! Read up to `limit` bytes into `str` (caller can allocate storage)
// void read(std::string &str, const size_t limit = std::numeric_limits<size_t>::max());

// //! Write a string, possibly blocking until all is written
// size_t write(const char *str, const bool write_all = true) { return write(BufferViewList(str), write_all); }

// //! Write a string, possibly blocking until all is written
// size_t write(const std::string &str, const bool write_all = true) { return write(BufferViewList(str), write_all); }

// //! Write a buffer (or list of buffers), possibly blocking until all is written
// size_t write(BufferViewList buffer, const bool write_all = true);

// //! Set blocking(true) or non-blocking(false)
// void set_blocking(const bool blocking_state);
