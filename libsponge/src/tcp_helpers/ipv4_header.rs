use std::fmt::{Debug, Display};

use crate::{InternetChecksum, NetParser, ParseError};

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
    cksum: u16,
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
    pub fn parse(p: &mut NetParser) -> Result<(), ParseError> {
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
        if p.is_err() { return p.get_result(); }
        let mut checksum = InternetChecksum::default();
        v.push(4*hlen);
        checksum.add(&v);
        if checksum.value() != 0 {
            return Err(ParseError::BadChecksum);
        }
        Ok(())
    }
    pub fn serialize;
    pub fn payload_length;
    pub fn pseudo_cksum;
}

impl Display for IPv4Header {
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


// //! Serialize the IPv4Header to a string (does not recompute the checksum)
// string IPv4Header::serialize() const {
//     // sanity checks
//     if (ver != 4) {
//         throw runtime_error("wrong IP version");
//     }
//     if (4 * hlen < IPv4Header::LENGTH) {
//         throw runtime_error("IP header too short");
//     }

//     string ret;
//     ret.reserve(4 * hlen);

//     const uint8_t first_byte = (ver << 4) | (hlen & 0xf);
//     NetUnparser::u8(ret, first_byte);  // version and header length
//     NetUnparser::u8(ret, tos);         // type of service
//     NetUnparser::u16(ret, len);        // length
//     NetUnparser::u16(ret, id);         // id

//     const uint16_t fo_val = (df ? 0x4000 : 0) | (mf ? 0x2000 : 0) | (offset & 0x1fff);
//     NetUnparser::u16(ret, fo_val);  // flags and offset

//     NetUnparser::u8(ret, ttl);    // time to live
//     NetUnparser::u8(ret, proto);  // protocol number

//     NetUnparser::u16(ret, cksum);  // checksum

//     NetUnparser::u32(ret, src);  // src address
//     NetUnparser::u32(ret, dst);  // dst address

//     ret.resize(4 * hlen);  // expand header to advertised size

//     return ret;
// }

// uint16_t IPv4Header::payload_length() const { return len - 4 * hlen; }

// //! \details This value is needed when computing the checksum of an encapsulated TCP segment.
// //! ~~~{.txt}
// //!   0      7 8     15 16    23 24    31
// //!  +--------+--------+--------+--------+
// //!  |          source address           |
// //!  +--------+--------+--------+--------+
// //!  |        destination address        |
// //!  +--------+--------+--------+--------+
// //!  |  zero  |protocol|  payload length |
// //!  +--------+--------+--------+--------+
// //! ~~~
// uint32_t IPv4Header::pseudo_cksum() const {
//     uint32_t pcksum = (src >> 16) + (src & 0xffff);  // source addr
//     pcksum += (dst >> 16) + (dst & 0xffff);          // dest addr
//     pcksum += proto;                                 // protocol
//     pcksum += payload_length();                      // payload length
//     return pcksum;
// }

// //! \returns A string with the header's contents
// std::string IPv4Header::to_string() const {
//     stringstream ss{};
//     ss << hex << boolalpha << "IP version: " << +ver << '\n'
//        << "IP hdr len: " << +hlen << '\n'
//        << "IP tos: " << +tos << '\n'
//        << "IP dgram len: " << +len << '\n'
//        << "IP id: " << +id << '\n'
//        << "Flags: df: " << df << " mf: " << mf << '\n'
//        << "Offset: " << +offset << '\n'
//        << "TTL: " << +ttl << '\n'
//        << "Protocol: " << +proto << '\n'
//        << "Checksum: " << +cksum << '\n'
//        << "Src addr: " << +src << '\n'
//        << "Dst addr: " << +dst << '\n';
//     return ss.str();
// }

// std::string IPv4Header::summary() const {
//     stringstream ss{};
//     ss << hex << boolalpha << "IPv" << +ver << ", "
//        << "len=" << +len << ", "
//        << "protocol=" << +proto << ", " << (ttl >= 10 ? "" : "ttl=" + ::to_string(ttl) + ", ")
//        << "src=" << inet_ntoa({htobe32(src)}) << ", "
//        << "dst=" << inet_ntoa({htobe32(dst)});
//     return ss.str();
// }
