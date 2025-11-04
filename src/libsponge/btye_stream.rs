use std::{collections::VecDeque, ops::Sub};

#[derive(Default, Debug)]
pub struct ByteStream {
    buffer: VecDeque<u8>,
    capacity: usize,
    end_write: bool,
    end_read: bool,
    written_bytes: usize,
    read_bytes: usize,
    _error: bool,
}

impl ByteStream {
    pub fn new(capa: usize) -> Self {
        ByteStream {
            capacity: capa,
            ..Default::default()
        }
    }

    pub fn write(&mut self, data: &str) -> usize {
        let real_write = (self.capacity.sub(self.buffer.len())).min(data.len());
        self.buffer.extend(data.bytes().take(real_write));
        self.written_bytes.checked_add(real_write).expect("written_bytes overflow!");
        real_write
    }

    pub fn peek_output(&self, len: usize) -> String {
        self.buffer
            .iter()
            .take(len.min(self.buffer.len()))
            .map(|&b| b as char)
            .collect::<String>()
    }

    pub fn pop_output(&mut self, len: usize) {
        if len > self.buffer.len() {
            self.set_error();
            return ;
        }

        self.buffer.drain(..len);
        self.read_bytes.checked_add(len).expect("read_bytes overflow!");
    }

    pub fn read(&mut self, len: usize) -> String {
        if len > self.buffer.len() {
            self.set_error();
            return String::new();
        }

        let bytes: Vec<u8> = self.buffer.drain(..len).collect();
        self.read_bytes.checked_add(len).expect("read_bytes overflow!");
        
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub fn end_input(&mut self) { self.end_write = true; }
    pub fn set_error(&mut self) { self._error = true; }
    pub fn remaining_capacity(&self) -> usize { self.capacity.sub(self.buffer.len()) }
    pub fn input_ended(&self) -> bool { self.end_write }
    pub fn error(&self) -> bool { self._error }
    pub fn buffer_size(&self) -> usize { self.buffer.len() }
    pub fn buffer_empty(&self) -> bool { self.buffer.is_empty() }
    pub fn eof(&self) -> bool { self.buffer_empty() && self.end_write }
    pub fn bytes_written(&self) -> usize { self.written_bytes }
    pub fn bytes_read(&self) -> usize { self.read_bytes }
}
