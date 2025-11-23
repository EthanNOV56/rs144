use crate::{Buffer, BufferList, IPv4Header, InternetChecksum, NetParser, ParseError};

#[derive(Default)]
struct IPv4Datagram {
    header: IPv4Header,
    payload: BufferList,
}

impl IPv4Datagram {
    pub fn try_parse(&mut self, buf: Buffer) -> Result<(), ParseError> {
        let mut p = NetParser::new(buf);
        self.header.try_parse(&mut p)?;
        self.payload = p.buffer_mut().take().into();

        if self.payload.len() != self.header.payload_length() as usize {
            return Err(ParseError::PacketTooShort);
        }

        p.get_result()
    }
    pub fn try_serialize(&self) -> Result<BufferList, ParseError> {
        if self.payload.len() != self.header.payload_length() as usize {
            return Err(ParseError::PayloadSizeMismatch);
        }

        let mut header_out = self.header;
        header_out.cksum = 0;
        let header_zero_checksum = header_out.try_serialize()?;

        let mut checksum = InternetChecksum::default();
        checksum.add(header_zero_checksum.as_slice());
        header_out.cksum = checksum.value();

        let mut ret = BufferList::default();
        ret.append(header_out.try_serialize()?.into());
        ret.append(self.payload.clone());

        Ok(ret)
    }

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
