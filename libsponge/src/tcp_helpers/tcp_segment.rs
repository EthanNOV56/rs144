use crate::{Buffer, BufferList, InternetChecksum, NetParser, ParseError, TCPHeader};

#[derive(Debug, Default, Clone)]
pub struct TCPSegment {
    header: TCPHeader,
    payload: Buffer,
}

impl TCPSegment {
    pub fn parse(
        &mut self,
        buffer: Buffer,
        datagram_layer_checksum: u32,
    ) -> Result<(), ParseError> {
        let mut checksum = InternetChecksum::new(datagram_layer_checksum);
        checksum.add(buffer.as_ref());
        if checksum.value() != 0 {
            return Err(ParseError::BadChecksum);
        }

        let mut p = NetParser::new(buffer);
        self.header.parse(&mut p);
        self.payload = p.get_buffer_mut().take();
        p.get_result()
    }

    pub fn serialize(&mut self, datagram_layer_checksum: u32) -> Result<BufferList, ParseError> {
        let mut header_out = self.header.clone();
        let mut check_sum = InternetChecksum::new(datagram_layer_checksum);
        let hr_ser = header_out.serialize()?;
        check_sum.add(&hr_ser);
        check_sum.add(&self.payload.as_ref());
        header_out.check_sum = check_sum.value();
        Ok(vec![hr_ser.into(), self.payload_mut().take()].into())
    }

    pub fn header(&self) -> &TCPHeader {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut TCPHeader {
        &mut self.header
    }

    pub fn payload(&self) -> &Buffer {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut Buffer {
        &mut self.payload
    }

    pub fn length_in_sequence_space(&self) -> usize {
        self.payload.size() + (self.header.syn as usize) + (self.header.fin as usize)
    }
}
