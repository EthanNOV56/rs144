use std::fmt::{Debug, Display};

use crate::{InternetChecksum, NetParser, NetUnparser, ParseError};

#[derive(Clone, Copy)]
pub struct IPv4Header {
    ver: u8,
    hlen: u8,
    tos: u8,
    len: u16,
    id: u16,
    df: bool,
    mf: bool,
    offset: u16,
    ttl: u8,
    proto: u8,
    pub cksum: u16,
    src: u32,
    dst: u32,
}

impl IPv4Header {
    const LENGTH: usize = 20;
    const DEFAULT_TTL: u8 = 128;
    const PROTO_TCP: u8 = 6;
}

impl Default for IPv4Header {
    fn default() -> Self {
        Self {
            ver: 4,
            hlen: Self::LENGTH as u8 / 4,
            tos: 0,
            len: 0,
            id: 0,
            df: true,
            mf: false,
            offset: 0,
            ttl: Self::DEFAULT_TTL,
            proto: Self::PROTO_TCP,
            cksum: 0,
            src: 0,
            dst: 0,
        }
    }
}

impl IPv4Header {
    #[allow(unused_variables)]
    pub fn try_parse(&self, p: &mut NetParser) -> Result<(), ParseError> {
        let original_serialized_version = p.buffer();
        let mut v = original_serialized_version.as_ref().to_vec();
        let len = original_serialized_version.len();
        if len < IPv4Header::LENGTH {
            return Err(ParseError::PacketTooShort);
        }

        let first_byte = p.parse_u8();
        let ver = first_byte >> 4;
        if ver != 4 {
            return Err(ParseError::WrongIPVersion);
        }

        let hlen = first_byte & 0x0f;
        if hlen < 5 {
            return Err(ParseError::HeaderTooShort);
        }
        let tos = p.parse_u8();
        let len = p.parse_u16();
        if (len as u8) < 4 * hlen {
            return Err(ParseError::PacketTooShort);
        }
        let id = p.parse_u16();
        let fo_val = p.parse_u16();
        let df = (fo_val & 0x4000) != 0;
        let mf = (fo_val & 0x2000) != 0;
        let offset = fo_val & 0x1fff;
        let ttl = p.parse_u8();
        let proto = p.parse_u8();
        let cksum = p.parse_u16();
        let src = p.parse_u32();
        let dst = p.parse_u32();

        if len != p.buffer().len() as _ {
            return Err(ParseError::TruncatedPacket);
        }

        p.remove_prefix(hlen as usize * 4 - Self::LENGTH);
        if p.is_err() {
            return p.get_result();
        }
        let mut checksum = InternetChecksum::default();
        v.push(4 * hlen);
        checksum.add(&v);
        if checksum.value() != 0 {
            return Err(ParseError::BadChecksum);
        }
        Ok(())
    }

    pub fn try_serialize(&self) -> Result<Vec<u8>, ParseError> {
        if self.ver != 4 {
            return Err(ParseError::WrongIPVersion);
        }
        if self.hlen as usize * 5 < Self::LENGTH {
            return Err(ParseError::HeaderTooShort);
        }

        let mut ret = Vec::with_capacity(4 * self.hlen as usize);
        let first_byte = (self.ver << 4) | (self.hlen & 0xf);
        NetUnparser::u8(&mut ret, first_byte);
        NetUnparser::u8(&mut ret, self.tos);
        NetUnparser::u16(&mut ret, self.len);
        NetUnparser::u16(&mut ret, self.id);
        let fo_val = if self.df { 0x4000 } else { 0 }
            | if self.mf { 0x2000 } else { 0 }
            | (self.offset & 0x1fff);
        NetUnparser::u16(&mut ret, fo_val);
        NetUnparser::u8(&mut ret, self.ttl);
        NetUnparser::u8(&mut ret, self.proto);
        NetUnparser::u16(&mut ret, self.cksum);
        NetUnparser::u32(&mut ret, self.src);
        NetUnparser::u32(&mut ret, self.dst);
        ret.resize(4 * self.hlen as usize, 0);
        Ok(ret)
    }

    #[inline(always)]
    pub fn payload_length(&self) -> u16 {
        self.len - self.hlen as u16
    }

    pub fn pseudo_cksum(&self) -> u32 {
        let mut pcksum = self.src >> 16 + self.src & 0xffff;
        pcksum += self.dst >> 16 + self.dst & 0xffff;
        pcksum += self.proto as u32;
        pcksum += self.payload_length() as u32;
        pcksum
    }
}

impl Display for IPv4Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPv{}, len={}, protocol={}, {}, src={}, dst={}",
            self.ver,
            self.len,
            self.proto,
            if self.ttl >= 10 {
                format!("")
            } else {
                format!("ttl={}", self.ttl)
            },
            self.src.to_be(),
            self.dst.to_be()
        )
    }
}

impl Debug for IPv4Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPv4 Header: Version={}, Header Length={}, TOS={}, Length={}, ID={}, DF={}, MF={}, Offset={}, TTL={}, Protocol={}, Checksum={}, Source={}, Destination={}",
            self.ver,
            self.hlen,
            self.tos,
            self.len,
            self.id,
            self.df,
            self.mf,
            self.offset,
            self.ttl,
            self.proto,
            self.cksum,
            self.src,
            self.dst
        )
    }
}
