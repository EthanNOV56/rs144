use crate::{Buffer, BufferList, IPv4Header, NetParser};

struct IPv4Datagram {
    header: IPv4Header,
    payload: BufferList,
}

// ParseResult IPv4Datagram::parse(const Buffer buffer) {

// }

// BufferList IPv4Datagram::serialize() const {
//     if (_payload.size() != _header.payload_length()) {
//         throw runtime_error("IPv4Datagram::serialize: payload is wrong size");
//     }

//     IPv4Header header_out = _header;
//     header_out.cksum = 0;
//     const string header_zero_checksum = header_out.serialize();

//     // calculate checksum -- taken over header only
//     InternetChecksum check;
//     check.add(header_zero_checksum);
//     header_out.cksum = check.value();

//     BufferList ret;
//     ret.append(header_out.serialize());
//     ret.append(_payload);
//     return ret;
// }

impl IPv4Datagram {
    pub fn parse(&mut self, buf: Buffer) -> Result<Self, ParseError> {
        let mut p = NetParser::new(buf);
        // self.header.pa
        // self.payload = p.buffer();

        //     NetParser p{buffer};
        //     _header.parse(p);
        //     _payload = p.buffer();

        //     if (_payload.size() != _header.payload_length()) {
        //         return ParseResult::PacketTooShort;
        //     }

        //     return p.get_error();
    }
    // pub fn serialize(&self) -> BufferList {}

    pub fn header(&self) -> &IPv4Header {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut IPv4Header {
        &mut self.header
    }

    pub fn payload(&self) -> &BufferList {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut BufferList {
        &mut self.payload
    }
}

pub type InternetDatagram = IPv4Datagram;
