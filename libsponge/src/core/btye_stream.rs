use crate::util::buffer::{BufferError, BufferList};

#[derive(Default, Debug)]
pub struct ByteStream {
    buffer: BufferList,
    capacity: usize,
    bytes_written: usize,
    bytes_read: usize,
    input_ended: bool,
    error: bool,
}

impl ByteStream {
    pub fn new(capacity: usize) -> Self {
        ByteStream {
            capacity,
            ..Default::default()
        }
    }

    pub fn read(&mut self, len: usize) -> Result<Vec<u8>, BufferError> {
        let vec = self.peek_out(len);
        self.pop_output(len)?;
        Ok(vec)
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let len = data.len().min(self.capacity - self.buffer.size());
        self.bytes_written += len;
        self.buffer.append(BufferList::from(data[..len].to_vec()));
        len
    }

    pub fn peek_out(&self, len: usize) -> Vec<u8> {
        let length = len.min(self.buffer.size());
        let vec: Vec<u8> = (&self.buffer).into(); // overhead
        vec[..length].to_vec()
    }

    pub fn pop_output(&mut self, len: usize) -> Result<Vec<u8>, BufferError> {
        let length = len.min(self.buffer.size());
        self.bytes_read += length;
        self.buffer.try_remove_prefix(length)
    }

    #[inline(always)]
    pub fn end_input(&mut self) {
        self.input_ended = true;
    }

    #[inline(always)]
    pub fn input_ended(&self) -> bool {
        self.input_ended
    }

    #[inline(always)]
    pub fn buffer_size(&self) -> usize {
        self.buffer.size()
    }

    #[inline(always)]
    pub fn buffer_empty(&self) -> bool {
        self.buffer.size() == 0
    }

    #[inline(always)]
    pub fn eof(&self) -> bool {
        self.buffer_empty() && self.input_ended()
    }

    #[inline(always)]
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }

    #[inline(always)]
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    #[inline(always)]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity - self.buffer.size()
    }

    #[inline(always)]
    pub fn error(&self) -> bool {
        self.error
    }

    #[inline(always)]
    pub fn set_error(&mut self) {
        self.error = true;
    }
}
