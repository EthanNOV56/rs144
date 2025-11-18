use crate::{Buffer, BufferList, EthernetHeader, NetParser, ParseError};

pub struct EthernetFrame {
    header: EthernetHeader,
    payload: BufferList,
}

// BufferList EthernetFrame::serialize() const {

// }

impl EthernetFrame {
    pub fn parse(&mut self, buf: Buffer) -> Result<(), ParseError> {
        let mut p = NetParser::new(buf);
        self.header.parse(&mut p)?;
        self.payload = p.get_buffer_mut().take().into();
        p.get_result()
    }

    pub fn serialize(&mut self) -> BufferList {
        let mut ret: BufferList = self.header.serialze().into();
        ret.append(self.payload.take());
        ret
    }

    pub fn header(&self) -> &EthernetHeader {
        &self.header
    }
    pub fn header_mut(&mut self) -> &EthernetHeader {
        &mut self.header
    }

    pub fn payload(&self) -> &BufferList {
        &self.payload
    }
    pub fn payload_mut(&mut self) -> &BufferList {
        &mut self.payload
    }
}
