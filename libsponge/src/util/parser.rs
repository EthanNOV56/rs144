use crate::util::buffer::Buffer;

use thiserror::Error;

use std::ops::{AddAssign, BitAnd, Shl, ShlAssign, Shr};

#[derive(Debug, Error, Clone, Copy)]
pub enum ParseError {
    #[error("Bad checksum")]
    BadChecksum,
    #[error("Not enough data to finish parsing")]
    PacketTooShort,
    #[error("Got a version of IP other than 4")]
    WrongIPVersion,
    #[error("Header length is shorter than minimum required")]
    HeaderTooShort,
    #[error("Packet length is shorter than header claims")]
    TruncatedPacket,
    #[error("Packet uses unsupported features")]
    Unsupported,
    #[error("Payload size mismatch")]
    PayloadSizeMismatch,
}

impl ParseError {
    pub fn as_string(&self) -> String {
        match self {
            ParseError::BadChecksum => "Bad checksum".to_string(),
            ParseError::PacketTooShort => "Not enough data to finish parsing".to_string(),
            ParseError::WrongIPVersion => "Got a version of IP other than 4".to_string(),
            ParseError::HeaderTooShort => {
                "Header length is shorter than minimum required".to_string()
            }
            ParseError::TruncatedPacket => {
                "Packet length is shorter than header claims".to_string()
            }
            ParseError::Unsupported => "Packet uses unsupported features".to_string(),
            ParseError::PayloadSizeMismatch => "Payload size mismatch".to_string(),
        }
    }
}

pub struct NetParser {
    buffer: Buffer,
    result: Result<(), ParseError>,
}

pub trait UnsignedInt:
    From<u8>
    + Copy
    + Shl<usize, Output = Self>
    + ShlAssign<usize>
    + AddAssign<Self>
    + Shr<usize, Output = Self>
    + BitAnd<Self, Output = Self>
    + PartialEq
    + Sized
{
    const BYTE_MASK: Self;

    fn to_u8(self) -> u8;
}

macro_rules! impl_unsigned_int {
    ($($t:ty),*) => {
        $(
            impl UnsignedInt for $t {
                const BYTE_MASK: Self = 0xFF;

                fn to_u8(self) -> u8 {
                    (self & Self::BYTE_MASK) as u8
                }
            }
        )*
    };
}

impl_unsigned_int!(u8, u16, u32);

impl NetParser {
    fn check_size(&mut self, size: usize) {
        if size > self.buffer.len() {
            self.set_result(Err(ParseError::PacketTooShort));
        }
    }

    fn parse_int<T: UnsignedInt>(&mut self) -> T {
        let len = std::mem::size_of::<T>();
        self.check_size(len);

        let mut ret = T::from(0);
        if self.buffer.is_empty() {
            return ret;
        }

        for i in 0..len {
            ret <<= 8;
            ret += self.buffer.at(i).into();
        }
        ret
    }

    pub fn new(buffer: Buffer) -> Self {
        NetParser {
            buffer,
            result: Ok(()),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub fn get_result(&self) -> Result<(), ParseError> {
        self.result
    }

    pub fn set_result(&mut self, result: Result<(), ParseError>) {
        self.result = result;
    }

    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    pub fn parse_u32(&mut self) -> u32 {
        self.parse_int()
    }

    pub fn parse_u16(&mut self) -> u16 {
        self.parse_int()
    }

    pub fn parse_u8(&mut self) -> u8 {
        self.parse_int()
    }

    pub fn remove_prefix(&mut self, mut n: usize) {
        self.check_size(n);
        if self.is_err() {
            return;
        }
        self.buffer.remove_prefix(&mut n);
    }
}

pub struct NetUnparser;

impl NetUnparser {
    #[inline(always)]
    pub fn unparse_int<T: UnsignedInt>(s: &mut Vec<u8>, val: T) {
        let len = std::mem::size_of::<T>();
        for i in 0..len {
            let shift_amount = (len - i - 1) * 8;
            let the_byte: u8 = (val >> shift_amount).to_u8();
            s.push(the_byte);
        }
    }

    #[inline(always)]
    pub fn u32(s: &mut Vec<u8>, val: u32) {
        Self::unparse_int(s, val);
    }

    #[inline(always)]
    pub fn u16(s: &mut Vec<u8>, val: u16) {
        Self::unparse_int(s, val);
    }

    #[inline(always)]
    pub fn u8(s: &mut Vec<u8>, val: u8) {
        Self::unparse_int(s, val);
    }
}
