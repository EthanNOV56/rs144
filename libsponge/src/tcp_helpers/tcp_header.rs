use std::fmt::{self, Debug, Display};

use crate::{
    WrappingU32,
    util::parser::{NetParser, NetUnparser, ParseError},
};

#[derive(Clone)]
pub struct TCPHeader {
    src_port: u16,
    dst_port: u16,
    pub seq_no: WrappingU32,
    ack_no: WrappingU32,
    data_offset: u8,
    urg: bool,
    ack: bool,
    psh: bool,
    rst: bool,
    pub syn: bool,
    pub fin: bool,
    win: u16,
    pub check_sum: u16,
    urg_ptr: u16,
}

impl Default for TCPHeader {
    fn default() -> Self {
        TCPHeader {
            src_port: 0,
            dst_port: 0,
            seq_no: WrappingU32::default(),
            ack_no: WrappingU32::default(),
            data_offset: Self::LENGTH as u8 / 4,
            urg: false,
            ack: false,
            psh: false,
            rst: false,
            syn: false,
            fin: false,
            win: 0,
            check_sum: 0,
            urg_ptr: 0,
        }
    }
}

impl TCPHeader {
    pub const LENGTH: usize = 20;

    pub fn parse(&mut self, p: &mut NetParser) -> Result<(), ParseError> {
        self.src_port = p.parse_u16(); // source port
        self.dst_port = p.parse_u16(); // destination port
        self.seq_no = WrappingU32::new(p.parse_u32()); // sequence number
        self.ack_no = WrappingU32::new(p.parse_u32()); // ack number
        self.data_offset = p.parse_u8() >> 4; // data offset
        let flags = p.parse_u8(); // byte including flags
        self.urg = flags & 0b0010_0000 != 0;
        self.ack = flags & 0b0001_0000 != 0;
        self.psh = flags & 0b0000_1000 != 0;
        self.rst = flags & 0b0000_0100 != 0;
        self.syn = flags & 0b0000_0010 != 0;
        self.fin = flags & 0b0000_0001 != 0;

        self.win = p.parse_u16(); // window size
        self.check_sum = p.parse_u16(); // checksum
        self.urg_ptr = p.parse_u16(); // urgent pointer

        if self.data_offset < 5 {
            return Err(ParseError::HeaderTooShort);
        }

        p.remove_prefix(self.data_offset as usize * 4 - TCPHeader::LENGTH);

        if p.is_err() {
            return p.get_result();
        }
        Ok(())
    }

    pub fn serialize(&self) -> Result<Vec<u8>, ParseError> {
        if self.data_offset < 5 {
            return Err(ParseError::HeaderTooShort);
        }

        let mut buf = Vec::with_capacity(4 * self.data_offset as usize);

        NetUnparser::u16(&mut buf, self.src_port); // source port
        NetUnparser::u16(&mut buf, self.dst_port); // destination port
        NetUnparser::u32(&mut buf, self.seq_no.raw_val()); // sequence number
        NetUnparser::u32(&mut buf, self.ack_no.raw_val()); // ack number
        NetUnparser::u8(&mut buf, self.data_offset << 4); // data offset

        let flags: u8 = (self.urg as u8) << 5
            | (self.ack as u8) << 4
            | (self.psh as u8) << 3
            | (self.rst as u8) << 2
            | (self.syn as u8) << 1
            | self.fin as u8;
        NetUnparser::u8(&mut buf, flags); // flags
        NetUnparser::u16(&mut buf, self.win); // window size
        NetUnparser::u16(&mut buf, self.check_sum); // checksum
        NetUnparser::u16(&mut buf, self.urg_ptr); // urgent pointer
        buf.resize(Self::LENGTH, 0);
        Ok(buf)
    }
}

impl PartialEq for TCPHeader {
    fn eq(&self, other: &TCPHeader) -> bool {
        self.src_port == other.src_port
            && self.dst_port == other.dst_port
            && self.data_offset == other.data_offset
            && self.urg == other.urg
            && self.ack == other.ack
            && self.psh == other.psh
            && self.rst == other.rst
            && self.syn == other.syn
            && self.fin == other.fin
            && self.win == other.win
            && self.check_sum == other.check_sum
            && self.urg_ptr == other.urg_ptr
    }
}

impl Display for TCPHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TCP source port: {:#06x}\n\
             TCP dest port: {:#06x}\n\
             TCP seqno: {}\n\
             TCP ackno: {}\n\
             TCP doff: {}\n\
             Flags: urg: {} ack: {} psh: {} rst: {} syn: {} fin: {}\n\
             TCP winsize: {}\n\
             TCP cksum: {:#06x}\n\
             TCP uptr: {}",
            self.src_port,
            self.dst_port,
            self.seq_no.raw_val(),
            self.ack_no.raw_val(),
            self.data_offset,
            self.urg,
            self.ack,
            self.psh,
            self.rst,
            self.syn,
            self.fin,
            self.win,
            self.check_sum,
            self.urg_ptr
        )
    }
}

impl Debug for TCPHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags = [
            if self.syn { "S" } else { "" },
            if self.ack { "A" } else { "" },
            if self.rst { "R" } else { "" },
            if self.fin { "F" } else { "" },
            if self.psh { "P" } else { "" },
            if self.urg { "U" } else { "" },
        ]
        .concat();

        write!(
            f,
            "TCPHeader(flags={}, seqno={}, ack={}, win={})",
            flags,
            self.seq_no.raw_val(),
            self.ack_no.raw_val(),
            self.win
        )
    }
}

impl TCPHeader {
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }

    pub fn summary(&self) -> String {
        format!("{:?}", self)
    }
}
