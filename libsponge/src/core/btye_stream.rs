use std::collections::VecDeque;

#[derive(Default, Debug)]
pub struct ByteStream {
    buffer: VecDeque<u8>,
    capacity: usize,
    end_read: bool,
    bytes_written: usize,
    bytes_read: usize,
    error: bool,
}

impl ByteStream {
    pub fn new(capacity: usize) -> Self {
        ByteStream {
            capacity,
            ..Default::default()
        }
    }
}
