use std::error::Error;
use std::fmt;
use std::io;
use std::ops::AddAssign;
use std::ops::MulAssign;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct TaggedError {
    attempt: String,
    source: io::Error,
}

impl TaggedError {
    pub fn new(attempt: impl Into<String>, error: io::Error) -> Self {
        Self {
            attempt: attempt.into(),
            source: error,
        }
    }

    pub fn unix(attempt: impl Into<String>) -> Self {
        Self::new(attempt, io::Error::last_os_error())
    }
}

impl fmt::Display for TaggedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.attempt, self.source)
    }
}

impl Error for TaggedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub fn system_call<F>(attempt: &str, f: F) -> Result<i32, TaggedError>
where
    F: FnOnce() -> i32,
{
    let result = f();
    if result < 0 {
        Err(TaggedError::unix(attempt))
    } else {
        Ok(result)
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Milliseconds(u64);

impl From<u64> for Milliseconds {
    fn from(millis: u64) -> Self {
        Milliseconds(millis)
    }
}

impl Into<u64> for Milliseconds {
    fn into(self) -> u64 {
        self.0
    }
}

impl AddAssign for Milliseconds {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl MulAssign<u64> for Milliseconds {
    fn mul_assign(&mut self, other: u64) {
        self.0 *= other;
    }
}

pub fn timestamp_ms() -> Milliseconds {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64)
        .into()
}

#[derive(Debug, Clone, Default)]
pub struct InternetChecksum {
    sum: u32,
}

impl InternetChecksum {
    pub fn new(initial_sum: u32) -> Self {
        Self { sum: initial_sum }
    }

    pub fn add(&mut self, data: &[u8]) {
        for chunk in data.chunks(2) {
            let word = if chunk.len() == 2 {
                u16::from_be_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], 0])
            };
            self.sum = self.sum.wrapping_add(word as u32);
        }
    }

    pub fn value(&self) -> u16 {
        let mut sum = self.sum;
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !(sum as u16)
    }
}

pub fn hexdump(data: &[u8], indent: usize) -> String {
    let indent_str = " ".repeat(indent);
    let mut output = String::new();

    for (i, chunk) in data.chunks(16).enumerate() {
        output.push_str(&format!("{}{:08x}: ", indent_str, i * 16));

        for (j, &byte) in chunk.iter().enumerate() {
            if j == 8 {
                output.push(' ');
            }
            output.push_str(&format!("{:02x} ", byte));
        }

        if chunk.len() < 16 {
            let spaces = (16 - chunk.len()) * 3 + if chunk.len() <= 8 { 1 } else { 0 };
            output.push_str(&" ".repeat(spaces));
        }

        output.push_str(" ");

        for &byte in chunk {
            output.push(if (32..127).contains(&byte) {
                byte as char
            } else {
                '.'
            });
        }

        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum() {
        let mut checksum = InternetChecksum::new(0);
        let data = [0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00];
        checksum.add(&data);
        let result = checksum.value();
        assert_eq!(result, 0xb861);
    }

    #[test]
    fn test_hexdump() {
        let data = b"Hello, World! This is a test.";
        let dump = hexdump(data, 2);
        assert!(dump.contains("4865 6c6c 6f2c 2057  "));
    }
}
