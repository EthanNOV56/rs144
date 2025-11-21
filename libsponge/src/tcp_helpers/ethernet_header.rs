use crate::{NetParser, NetUnparser, ParseError};

use itertools::Itertools;

use std::{
    fmt::{self, Display, Formatter},
    slice::{Iter, IterMut},
};

pub struct EthernetAddress([u8; 6]);
pub const ETHERNETBROADCAST: EthernetAddress = EthernetAddress([0xff; 6]);

impl EthernetAddress {
    pub fn iter(&self) -> Iter<'_, u8> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, u8> {
        self.0.iter_mut()
    }
}

pub struct EthernetHeader {
    pub dst: EthernetAddress,
    pub src: EthernetAddress,
    pub ty: u16,
}

impl EthernetHeader {
    const LENGTH: usize = 14;
    const TYPE_IPV4: u16 = 0x800;
    const TYPE_ARP: u16 = 0x806;

    pub fn parse(&mut self, p: &mut NetParser) -> Result<(), ParseError> {
        if p.buffer().len() < Self::LENGTH {
            return Err(ParseError::PacketTooShort);
        }
        self.dst.iter_mut().for_each(|byte| *byte = p.parse_u8());
        self.src.iter_mut().for_each(|byte| *byte = p.parse_u8());
        self.ty = p.parse_u16();
        p.get_result()
    }

    pub fn serialze(&self) -> Vec<u8> {
        let mut ser = Vec::with_capacity(Self::LENGTH);
        self.dst.iter().for_each(|&b| NetUnparser::u8(&mut ser, b));
        self.src.iter().for_each(|&b| NetUnparser::u8(&mut ser, b));
        NetUnparser::u16(&mut ser, self.ty);
        ser
    }
}

impl Display for EthernetAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.iter()
                .format_with(":", |b, f| { f(&format_args!("{:02x}", b)) })
        )
    }
}

impl Display for EthernetHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dst={}, src={}, type={}",
            self.dst,
            self.src,
            match self.ty {
                Self::TYPE_IPV4 => "IPV4",
                Self::TYPE_ARP => "ARP",
                _ => return write!(f, "[unknown type {:#06x}!]", self.ty),
            }
        )
    }
}
